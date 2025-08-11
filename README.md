# YaKS: Yet-another Kemono Scraper

It downloads content from Kemono.

[![[DOWNLOAD]](https://img.shields.io/badge/DOWNLOAD-Windows--x86__x64-blue)](https://github.com/dec32/yaks/releases/download/nightly/yaks-windows-x86_64.exe)

[![[DOWNLOAD]](https://img.shields.io/badge/DOWNLOAD-Linux--x86__x64-blue)](https://github.com/dec32/yaks/releases/download/nightly/yaks-linux-x86_64)

(I haven't figured out how to build the mac one)

## Why another one?

The popular ones do not support arranging files into different folders with custom names, have laggy and buggy interfaces and are kinda slow.

## How to use it?

Just use the command:

```Bash
yaks $URL
```

to download all the files from an artist.

You can filter out posts by their IDs using `--range`/`-r`

```Bash
# The ranges are left-closed, like [a, b)
yaks $URL --range ..67890
yaks $URL --range 12345..
yaks $URL --range 12345..67890
```

By default the files are saved to `$HOME/Downloads`, named as `{post_id}_{index}` (extentions are automatically handled) and goes in to a folder named by the artist's nickname.

If you want to save the files elsewhere, use `--out`/`-o`:

```Bash
yaks $URL --out /i/want/them/saved/here
```

If you want name and arrange the files differently, use `--template`/`-t`:

```Bash
yaks $URL --template {nickname}/{title}_{filename}
```

The supported placeholders are:
- `nickname`, `username` and `user_id` for artists
- `post_id` and `title` for posts
- `filename` and `index` for files

You probably don't need to adjust the level of concurrency, but `--jobs`/`-j` controls that.

```Bash
# My internet is super fast and I am not afraid of 429 Too Many Request.
yaks $URL --j255
```

## You say I need to type the arguments every single time?

No, you can create a configuration file called `conf.toml` in `%APPDATA%/yaks` and save your prefered arguments there:

```toml
# conf.toml
out = "/my/unholy/vault"
template = "{username}/{title}/{filename}"
jobs = 16
```

## But I want a GUI

I am working on it.
