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

// For the top banners. They don't have a visible bar
progress_style! {
    fetch_profile,
    "{spinner:.blue} {msg}",
    "/-\\| "
}
progress_style! {
    scrape_posts,
    "{spinner:.blue} [{pos}] {msg}",
    "◴◷◶◵ "
}
progress_style! {
    create_jobs,
    "{spinner:.blue} [{pos}/{len}] {msg}",
    "◴◷◶◵ "
}
progress_style! {
    download,
    "{spinner:.blue} [{pos}/{len}] {msg}",
    "◴◷◶◵ "
}
progress_style! {
    clear,
    "{spinner:.green} [{pos}/{len}] {msg}",
    "✓✓"
}
progress_style! {
    error,
    "{spinner:.red} [{pos}/{len}] {msg}",
    "!!"
}
// for progress bars
progress_style! {
    enqueued,
    "{spinner:.dim} {msg:<20} [{elapsed_precise}] [{wide_bar:.dim/dim}]",
    "◜◠◝◞◡◟ "
}
progress_style! {
    running,
    "{spinner:.green} {msg:<20} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})"
}
progress_style! {
    failed,
    "{spinner:.red} {msg:<20} [{elapsed_precise}] [{wide_bar:.red/blue}] {bytes}/{total_bytes}",
    "!!"
}
