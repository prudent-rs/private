use core::error::Error;
use file::{NAME_PREFIX, RND_PART_LEN};
use random_str::random::{CharBuilder, RandomStringBuilder};

use std::env;
use std::fs::File;
use std::path::Path;

#[path = "src/file.rs"]
mod file;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;

    let existing_rnd_part = file::existing_rnd_part(&out_dir);
    match existing_rnd_part {
        Ok(Some(_)) => (),
        Ok(None) => {
            let rnd_part = RandomStringBuilder::new()
                .with_length(RND_PART_LEN)
                .with_lowercase()
                .build()
                .ok_or_else(|| "Couldn't generate a random identifier.")?;
            assert_eq!(rnd_part.len(), RND_PART_LEN);

            let file_path = Path::new(&out_dir).join(format!("{NAME_PREFIX}{rnd_part}"));
            let _ = File::create(&file_path)?; // @TODO try without let _ =

            let new_rnd_part = file::existing_rnd_part(&out_dir);
            let new_rnd_part =
                match new_rnd_part {
                    Ok(Some(rnd_part)) => rnd_part,
                    Ok(None) => return Err(
                        "Creating a file seemed to succeed, but it couldn't be found afterwards."
                            .into(),
                    ),
                    Err(e) => return Err(e),
                };
            if rnd_part != new_rnd_part {
                return Err(format!("Created file with random part of its name: {rnd_part}, but then found a file with a different random part of its name: {new_rnd_part}.").into());
            }
        }
        Err(e) => return Err(e),
    }

    // cargo will re-run if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
