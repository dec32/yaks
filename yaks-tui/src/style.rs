macro_rules! progress_style {
    ($name:ident, $template:expr, $tick_chars:expr) => {
        pub fn $name() -> indicatif::ProgressStyle {
            use std::sync::OnceLock;

            use indicatif::ProgressStyle;
            static INSTANCE: OnceLock<ProgressStyle> = OnceLock::new();
            INSTANCE
                .get_or_init(|| {
                    ProgressStyle::default_bar()
                        .template($template)
                        .unwrap()
                        .progress_chars("#>-")
                        .tick_chars($tick_chars)
                })
                .clone()
        }
    };
    ($name:ident, $template:expr) => {
        pub fn $name() -> indicatif::ProgressStyle {
            use std::sync::OnceLock;

            use indicatif::ProgressStyle;
            static INSTANCE: OnceLock<ProgressStyle> = OnceLock::new();
            INSTANCE
                .get_or_init(|| {
                    ProgressStyle::default_bar()
                        .template($template)
                        .unwrap()
                        .progress_chars("#>-")
                })
                .clone()
        }
    };
}

// for banners
progress_style! {
    fetch_profile,
    "{spinner:.blue} {msg}",
    "?¿ "
}
progress_style! {
    scrape_posts,
    "{spinner:.blue} {msg}",
    "◴◷◶◵ "
}
progress_style! {
    collect_files,
    "{spinner:.blue} [{pos}/{len}] {msg}",
    "◴◷◶◵ "
}
progress_style! {
    download,
    "{spinner:.blue} [{pos}/{len}] {msg}",
    "◴◷◶◵ "
}
progress_style! {
    error,
    "{spinner:.red} [{pos}/{len}] {msg}",
    "◴◷◶◵ "
}
progress_style! {
    finish_with_error,
    "{spinner:.red} {msg}",
    "!!"
}
progress_style! {
    finish,
    "{spinner:.green} {msg}",
    "✓✓"
}
// for progress bars
progress_style! {
    enqueued,
    "{spinner:.dim} {msg:<20} [{elapsed_precise}] [{wide_bar:.dim/dim}]",
    "◜◠◝◞◡◟ "
}
progress_style! {
    running,
    "{spinner:.blue} {msg:<20} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})"
}
// for speed
progress_style! {
    speed,
    "({bytes_per_sec})"
}
