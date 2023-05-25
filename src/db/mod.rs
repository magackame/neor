use crate::session::User as AuthUser;
use crate::types::comment::{Comment, RawComment};
use crate::types::id::Id;
use crate::types::post::tags::Tags;
use crate::types::post::{Post, Preview as PostPreview, RawPost, RawPreview as RawPostPreview};
use crate::types::user::{RawUser, User};
use sqlx::mysql::MySqlPool;
use sqlx::mysql::MySqlQueryResult;

pub async fn fetch_user_by_username(
    db_pool: &MySqlPool,
    username: &str,
    user: impl Into<Option<&AuthUser>>,
) -> sqlx::Result<Option<User>> {
    sqlx::query_as!(
        RawUser,
        "
        SELECT
            users.id,
            users.username,
            users.role,
            CONCAT(files.id, \".\", files.extension) AS pfp,
            users.name,
            users.description,
            users.joined_at
        FROM users
            LEFT JOIN files ON users.pfp_file_id = files.id
        WHERE
            users.username = ?
        ",
        username
    )
    .fetch_optional(db_pool)
    .await
    .map(|result| result.map(|raw| User::from_raw(raw, user)))
}

pub async fn fetch_comment_by_id(
    db_pool: &MySqlPool,
    id: Id,
    user: &AuthUser,
) -> sqlx::Result<Option<Comment>> {
    sqlx::query_as!(
        RawComment,
        "
        SELECT
            comments.id,
            comments.post_id,
            comments.reply_to_comment_id,
            users_reply_to.id AS reply_to_user_id,
            users_reply_to.username AS reply_to_username,
            CONCAT(files_reply_to.id, \".\", files_reply_to.extension) AS reply_to_mini_pfp,
            comments.markdown_content AS content,
            users_posted_by.id AS posted_by_user_id,
            users_posted_by.username AS posted_by_username,
            CONCAT(files_posted_by.id, \".\", files_posted_by.extension) AS posted_by_mini_pfp,
            comments.posted_at,
            comments.modified_at
        FROM comments
            LEFT JOIN comments AS comments_reply_to ON comments.reply_to_comment_id = comments_reply_to.id
            LEFT JOIN users AS users_reply_to ON comments_reply_to.posted_by_user_id = users_reply_to.id
            LEFT JOIN files AS files_reply_to ON users_reply_to.mini_pfp_file_id = files_reply_to.id
            LEFT JOIN users AS users_posted_by ON comments.posted_by_user_id = users_posted_by.id
            LEFT JOIN files AS files_posted_by ON users_posted_by.mini_pfp_file_id = files_posted_by.id
        WHERE
            comments.id = ?
        ",
        id
    ).fetch_optional(db_pool).await.map(|result| result.map(|raw_comment| Comment::from_raw(raw_comment, user)))
}

pub async fn fetch_contentless_post_by_id(
    db_pool: &MySqlPool,
    post_id: Id,
    user: &AuthUser,
) -> sqlx::Result<Option<Post>> {
    let Some(raw_post) = fetch_raw_contentless_post_by_id(db_pool, post_id).await? else {
        return Ok(None);
    };

    let post = fetch_post_from_raw(db_pool, raw_post, user).await?;

    Ok(Some(post))
}

pub async fn fetch_raw_contentless_post_by_id(
    db_pool: &MySqlPool,
    post_id: Id,
) -> sqlx::Result<Option<RawPost>> {
    sqlx::query_as!(
        RawPost,
        "
        SELECT
            posts.id,
            posts.title,
            posts.description,
            '' AS content,
            users.id AS posted_by_user_id,
            users.username AS posted_by_username,
            CONCAT(files.id, \".\", files.extension) AS posted_by_mini_pfp,
            posts.posted_at,
            posts.modified_at
        FROM posts
            LEFT JOIN users ON posts.posted_by_user_id = users.id
            LEFT JOIN files ON users.mini_pfp_file_id = files.id
        WHERE
            posts.id = ?
        ",
        post_id
    )
    .fetch_optional(db_pool)
    .await
}

pub async fn insert_post_tags(
    db_pool: &MySqlPool,
    tags: &Tags,
    post_id: Id,
    user_id: Id,
) -> sqlx::Result<()> {
    for tag in tags {
        let tag_id = upsert_tag(db_pool, tag, user_id).await?;

        insert_post_tag(db_pool, post_id, tag_id).await?;
    }

    Ok(())
}

async fn insert_post_tag(
    db_pool: &MySqlPool,
    post_id: Id,
    tag_id: Id,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        INSERT INTO post_tags
        (
            post_id,
            tag_id
        )
        VALUES
        (
            ?,
            ?
        )
        ",
        post_id,
        tag_id
    )
    .execute(db_pool)
    .await
}

async fn upsert_tag(db_pool: &MySqlPool, tag: &str, created_by_user_id: Id) -> sqlx::Result<Id> {
    // TODO: Batch this up
    #[derive(Debug)]
    struct Tag {
        id: Id,
    }

    if let Some(tag) = sqlx::query_as!(
        Tag,
        "
        SELECT
            id
        FROM tags
        WHERE
            name = ?
        ",
        tag
    )
    .fetch_optional(db_pool)
    .await?
    {
        return Ok(tag.id);
    }

    let tag_insert_result = sqlx::query!(
        "
        INSERT INTO tags
        (
            name,
            created_by_user_id,
            created_at
        )
        VALUES
        (
            ?,
            ?,
            NOW()
        )
        ",
        tag,
        created_by_user_id
    )
    .execute(db_pool)
    .await?;

    Ok(tag_insert_result.last_insert_id())
}

pub async fn fetch_post_from_raw<'a>(
    db_pool: &MySqlPool,
    raw_post: RawPost,
    user: impl Into<Option<&'a AuthUser>>,
) -> sqlx::Result<Post> {
    let tags = fetch_post_tags(db_pool, raw_post.id).await?;

    let post = Post::from_raw(raw_post, tags, user);

    Ok(post)
}

pub async fn fetch_post_preview_from_raw(
    db_pool: &MySqlPool,
    raw_post_preview: RawPostPreview,
) -> sqlx::Result<PostPreview> {
    let tags = fetch_post_tags(db_pool, raw_post_preview.id).await?;

    let post = PostPreview::from_raw(raw_post_preview, tags);

    Ok(post)
}

async fn fetch_post_tags(db_pool: &MySqlPool, post_id: Id) -> sqlx::Result<Vec<String>> {
    #[derive(Debug)]
    struct Tag {
        pub name: String,
    }

    sqlx::query_as!(
        Tag,
        "
        SELECT
            tags.name
        FROM post_tags
            JOIN tags on post_tags.tag_id = tags.id
        WHERE
            post_tags.post_id = ?
        ",
        post_id
    )
    .fetch_all(db_pool)
    .await
    .map(|result| result.into_iter().map(|tag| tag.name).collect())
}
