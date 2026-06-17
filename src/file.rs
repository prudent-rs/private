use core::error::Error;
use std::fs;

pub const NAME_PREFIX: &str = "private_prefix_";
const NAME_PREFIX_LEN: usize = NAME_PREFIX.len();
pub const RND_PART_LEN: usize = 32;
const NAME_LEN: usize = NAME_PREFIX_LEN + RND_PART_LEN;

pub fn existing_rnd_part(out_dir: &str) -> Result<Option<String>, Box<dyn Error>> {
    let mut found_rnd_part = None;

    for dir_entry in fs::read_dir(out_dir)? {
        let dir_entry = dir_entry?;
        let file_type = dir_entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let file_name = dir_entry.file_name().into_string();
        let mut file_name = match file_name {
            Ok(file_name) => file_name,
            Err(_) => continue,
        };
        if file_name.len() != NAME_LEN || !file_name.starts_with(NAME_PREFIX) {
            continue;
        }
        file_name.replace_range(..NAME_PREFIX_LEN, "");
        let rnd_part = file_name;
        assert_eq!(rnd_part.len(), RND_PART_LEN);
        if found_rnd_part.is_some() {
            return Err(format!("There are two (or more) files under {out_dir} whose names start with {NAME_PREFIX} and whose names have same expected length.").into());
        }
        found_rnd_part = Some(rnd_part);
    }
    Ok(found_rnd_part)
}
