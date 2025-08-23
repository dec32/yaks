use cow_utils::CowUtils;
use std::borrow::Cow;

pub trait StrExt<'a> {
    fn to_path_safe(&'a self) -> Cow<'a, str>;
}

impl<'a> StrExt<'a> for str {
    fn to_path_safe(&'a self) -> Cow<'a, str> {
        self.cow_replace("/", "／")
    }
}
