use actix_files::NamedFile;
use actix_web::{get, Error, HttpRequest};

#[get("/files/{filename:(default|not_found|[0-9]+)\\.(jpeg|jpg|png|gif)}")]
pub async fn service(req: HttpRequest) -> Result<NamedFile, Error> {
    let filename = req
        .match_info()
        .query("filename")
        .parse::<String>()
        .unwrap();

    let path = format!("public/files/{filename}");

    let file = NamedFile::open(path)?;

    Ok(file)
}
