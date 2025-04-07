pub mod random_image;
pub mod categorized_image;

#[expect(deprecated)]
pub use random_image::{random_image, worker};

pub use categorized_image::cat_image;