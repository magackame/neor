# neor

Mix of Reddit, Hacker News and 4chan

## Features

- Post stuff
- Comment stuff
- Edit/Anonymise posts/comments
- Delete posts/comments (only mods and admins)
- Customize profile name, description and avatar
- Tag system
- Search posts by title
- Comment permalinks

## Building from source

### Linux

To build you first need to setup the environment and DB

Run `cp example-dotenv .env` and enter appropriate settings in `.env`

Run the migrations `mysql -u root -p < db/scheme.sql`

Then build with dynamic linking

`cargo b --release`

or with static linking

`cargo b --release target x86_64-unknown-linux-musl`

The resulting binary has HTML-templates and CSS baked in.
It only need a running MySQL instance with appropriate DB and
`public/files` folder, which will contain user avatars

Also host must have `ImageMagick` installed (specifically the `convert` function is
used for image resizing and conversion)

For SSL you can use a reverse proxy (Maybe I will add SSL support some day)

> Other platforms are just untested, maybe it will work

## WIP

+ Refactor
+ Make the API more type safe and testable
+ Add tests
+ Add email notifications and related user settings
+ Flow typograhpy
