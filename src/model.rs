use regex::Regex;

pub const ABS_NUM_STR: &str = r"(.*) (?:increased|decreased) from (\S*) to (\S*)";
pub const REL_NUM_STR: &str = r"(.*) (increased|decreased) by (\S*$)";
pub const ABS_TXT_STR: &str = r"(.*Talent) (.*) replaced with (.*)";
pub const NEW_NUM_STR: &str = r".*ow has a (\S*) ([^,]*)";

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum ChangeData {
    AbsoluteChange(String, String),
    RelativeChange(i32),
    OtherChange(String)
}

impl ChangeData {
    fn variant_eq(a: &ChangeData, b: &ChangeData) -> bool {
        std::mem::discriminant(a) == std::mem::discriminant(b)
    }

    fn diff(old: &ChangeData, new:&ChangeData) -> Result<ChangeData, String> {
        if ChangeData::variant_eq(old, new) {
            if let (ChangeData::AbsoluteChange(old_data, _), ChangeData::AbsoluteChange(_, new_data)) = (old, new) {
                return Ok(ChangeData::AbsoluteChange(old_data.to_string(), new_data.to_string()))
            } else if let (ChangeData::RelativeChange(old_data), ChangeData::RelativeChange(new_data)) = (old, new) {
                return Ok(ChangeData::RelativeChange(old_data + new_data))
            } else {
                return Err("ChangeData::OtherChange does not track diff".to_string())
            }
        } else {
            return Err("ChangeData variants are not equal".to_string())
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct PatchChange {
    property: String,
    version: String,
    data: ChangeData
}

impl PatchChange {
    fn new(property: &String, version: &String, data: ChangeData) -> Self {
        PatchChange {
            property: property.to_string(),
            version: version.to_string(),
            data
        }
    }

    fn diff(old: &PatchChange, new: &PatchChange) -> Result<PatchChange, String>{
        if old.property == new.property {
            Ok(PatchChange::new(&old.property, &new.version, ChangeData::diff(&old.data, &new.data)?))
        } else {
            Err("PatchChange.property values do not match".to_string())
        }
    }

    pub fn parse_text(change_line: &str, tree_location: String, version: &str) -> PatchChange {
        let abs_num_change = Regex::new(ABS_NUM_STR).unwrap();
        let rel_num_change = Regex::new(REL_NUM_STR).unwrap();
        let abs_txt_change = Regex::new(ABS_TXT_STR).unwrap();
        let new_num_change = Regex::new(NEW_NUM_STR).unwrap();

        if abs_num_change.is_match(&change_line) {
            let capture_groups = abs_num_change.captures(&change_line).unwrap();
            let mut property = tree_location;
            property.push_str(" > ");
            property.push_str(capture_groups.get(1).unwrap().as_str());
            let data = ChangeData::AbsoluteChange(
                capture_groups.get(2).unwrap().as_str().to_string(),
                capture_groups.get(3).unwrap().as_str().to_string()
            );
            return PatchChange::new(&property, &version.to_string(), data);
        
        } else if rel_num_change.is_match(&change_line) {
            let capture_groups = rel_num_change.captures(&change_line).unwrap();
            
            let mut shift_sign = 1;
            if capture_groups.get(2).unwrap().as_str() == "decreased" {
                shift_sign = -1;
            }

            let mut property = tree_location;
            property.push_str(" > ");
            property.push_str(capture_groups.get(1).unwrap().as_str());

            let data = ChangeData::RelativeChange(shift_sign * capture_groups.get(3).unwrap().as_str().parse::<i32>().unwrap());
            return PatchChange::new(&property, &version.to_string(), data);

        } else if abs_txt_change.is_match(&change_line) {
            let capture_groups = abs_txt_change.captures(&change_line).unwrap();
            let mut property = tree_location;
            property.push_str(" > ");
            property.push_str(capture_groups.get(1).unwrap().as_str());
            let data = ChangeData::AbsoluteChange(
                capture_groups.get(2).unwrap().as_str().to_string(),
                capture_groups.get(3).unwrap().as_str().to_string()
            );
            return PatchChange::new(&property, &version.to_string(), data);

        } else if new_num_change.is_match(&change_line) {
            let capture_groups = new_num_change.captures(&change_line).unwrap();
            let mut property = tree_location;
            property.push_str(" > ");
            property.push_str(capture_groups.get(2).unwrap().as_str());
            let data = ChangeData::AbsoluteChange(
                "0".to_string(),
                capture_groups.get(1).unwrap().as_str().to_string()
            );
            return PatchChange::new(&property, &version.to_string(), data)
        } else {
            let data = ChangeData::OtherChange(change_line.to_string());
            return PatchChange::new(&tree_location, &version.to_string(), data);
        }
    }

    pub fn write_text(&self) -> String {
        let property = &self.property;
        let data = &self.data;

        match data {
            ChangeData::AbsoluteChange(old, new) => {
                if property.contains("Talent") {
                    return format!("{} {} replaced with {}", property, old, new)
                } else {
                    let direction = absolute_change_direction(old, new);
                    return format!("{} {} from {} to {}", property, direction, old, new)
                }
            },
            ChangeData::RelativeChange(value) => {
                let mut direction = "increased".to_string();
                if value < &0 {
                    direction = "decreased".to_string()
                } else if value == &0 {
                    return format!("{} unchanged", property)
                }
                return format!("{} {} by {}", property, direction, value)
            },
            ChangeData::OtherChange(value) => {
                return format!("{} > {}", property, value)
            }
        }
    }
}

pub fn patch_diff(mut combined_patches: Vec<PatchChange>) -> Vec<PatchChange> {
    combined_patches.sort();

    let total_changes = &combined_patches.len();
    let mut change_iter = combined_patches.into_iter();
    let mut result: Vec<PatchChange> = vec![];
    let mut result_idx = 0;
    
    for combined_idx in 0..*total_changes {
        let current_change = change_iter.next().unwrap();

        if combined_idx == 0 {
            result.push(current_change);
            continue
        }

        let diff_result = PatchChange::diff(&result[result_idx], &current_change);

        match diff_result {
            Ok(diff_value) => {
                result[result_idx] = diff_value;
            },
            Err(_) => {
                result.push(current_change);
                result_idx += 1;
            }
        }
    }
    return result;
}

fn absolute_change_direction(old: &String, new: &String) -> String {
    let num_match = Regex::new(r"([0-9\.]+)").unwrap();
    let old_values: Vec<&str> = old.split("/").collect();
    let new_values: Vec<&str> = new.split("/").collect();
    let mut longest_len = old_values.len();

    if new_values.len() > longest_len {
        longest_len = new_values.len()
    }

    let mut i = 0;

    let mut is_inc = false;
    let mut is_dec = false;

    while i < longest_len {
        let mut old_str = old_values[old_values.len() - 1];
        if i < old_values.len() {
            old_str = old_values[i];
        }
        let old_value = num_match.captures(&old_str).unwrap().get(0).unwrap().as_str().parse::<f32>().unwrap();
        let mut new_str = new_values[new_values.len() - 1];
        if i < new_values.len() {
            new_str = new_values[i];
        }
        let new_value = num_match.captures(&new_str).unwrap().get(0).unwrap().as_str().parse::<f32>().unwrap();
        if new_value < old_value && is_dec == false {
            is_dec = true
        }
        if new_value > old_value && is_inc == false {
            is_inc = true
        }
        i += 1;
    }

    if is_inc && is_dec {
            return "rescaled".to_string()
        }
    if is_inc {
        return "increased".to_string()
    }
    if is_dec {
        return "decreased".to_string()
    }
    
    return "changed".to_string()
}

#[cfg(test)]
mod tests {
    use crate::model::{ChangeData, PatchChange, patch_diff, absolute_change_direction};

    #[test]
    fn absolute_diff_works() {
        let old_change = ChangeData::AbsoluteChange("A".to_string(), "B".to_string());
        let new_change = ChangeData::AbsoluteChange("B".to_string(), "C".to_string());
        let result = ChangeData::diff(&old_change, &new_change).unwrap();
        assert_eq!(ChangeData::AbsoluteChange("A".to_string(), "C".to_string()), result)
    }

    #[test]
    fn relative_diff_works() {
        let old_change = ChangeData::RelativeChange(1);
        let new_change = ChangeData::RelativeChange(2);
        let result = ChangeData::diff(&old_change, &new_change).unwrap();
        assert_eq!(ChangeData::RelativeChange(3), result)
    }

    #[test]
    fn other_diff_fails() {
        let old_change = ChangeData::OtherChange("Change 1".to_string());
        let new_change = ChangeData::OtherChange("Change 2".to_string());
        let result = ChangeData::diff(&old_change, &new_change).err().unwrap();
        assert_eq!("ChangeData::OtherChange does not track diff".to_string(), result)
    }

    #[test]
    fn different_variants_fail_diff() {
        let old_change = ChangeData::AbsoluteChange("A".to_string(), "B".to_string());
        let new_change = ChangeData::RelativeChange(10);
        let result = ChangeData::diff(&old_change, &new_change).err().unwrap();
        assert_eq!("ChangeData variants are not equal".to_string(), result)
    }

    #[test]
    fn same_property_diff_works() {
        let old_property = "Items > Blade Mail > Duration".to_string();
        let old_patch_name = "7.32".to_string();
        let old_data = ChangeData::AbsoluteChange("4.5s".to_string(), "5.5s".to_string());
        let old_change = PatchChange::new(&old_property, &old_patch_name, old_data);

        let new_data = ChangeData::AbsoluteChange("5.5s".to_string(), "6.5s".to_string());
        let new_patch_name = "7.32a".to_string();
        let new_change = PatchChange::new(&old_property, &new_patch_name, new_data);

        let result = PatchChange::diff(&old_change, &new_change).unwrap();

        assert_eq!(PatchChange::new(&old_property, &new_patch_name, ChangeData::AbsoluteChange("4.5s".to_string(), "6.5s".to_string())), result)
    }

    #[test]
    fn different_properties_fail_diff() {
        let old_property = "Items > Blade Mail > Duration".to_string();
        let old_patch_name = "7.32".to_string();
        let old_data = ChangeData::AbsoluteChange("4.5s".to_string(), "5.5s".to_string());
        let old_change = PatchChange::new(&old_property, &old_patch_name, old_data);

        let new_property = "Items > Black King Bar > Duration".to_string();
        let new_patch_name = "7.32a".to_string();
        let new_data = ChangeData::AbsoluteChange("5.5s".to_string(), "6.5s".to_string());
        let new_change = PatchChange::new(&new_property, &new_patch_name, new_data);

        let result = PatchChange::diff(&old_change, &new_change).err().unwrap();

        assert_eq!("PatchChange.property values do not match".to_string(), result)
    }

    #[test]
    fn patch_diff_works() {
        let old_patch_name = "7.32".to_string();
        let old_change_1 = PatchChange::new(
            &"Items > Blade Mail > Duration".to_string(),
            &old_patch_name,
            ChangeData::AbsoluteChange("4.5s".to_string(), "5.5s".to_string()));
        let old_change_2 = PatchChange::new(
            &"Items > Blade Mail > Armor".to_string(),
            &old_patch_name,
            ChangeData::AbsoluteChange("4".to_string(), "5".to_string()));
        let old_change_3 = PatchChange::new(
            &"Heroes > Zeus > Base Armor".to_string(),
            &old_patch_name,
            ChangeData::RelativeChange(2));
        let old_change_4 = PatchChange::new(
            &"Heroes > Zeus".to_string(),
            &old_patch_name,
            ChangeData::OtherChange("Something random".to_string()));
        let mut old_patch = vec![old_change_1, old_change_2, old_change_3, old_change_4];

        let new_patch_name = "7.32a".to_string();
        let new_change_1 = PatchChange::new(
            &"Items > Blade Mail > Duration".to_string(),
            &new_patch_name,
            ChangeData::AbsoluteChange("5.5s".to_string(), "6.5s".to_string()));
        let new_change_2 = PatchChange::new(
            &"Items > Blade Mail > Damage".to_string(),
            &new_patch_name,
            ChangeData::AbsoluteChange("11".to_string(), "60".to_string()));
        let new_change_3 = PatchChange::new(
            &"Heroes > Zeus > Base Armor".to_string(),
            &new_patch_name,
            ChangeData::RelativeChange(1));
        let new_change_4 = PatchChange::new(
            &"Heroes > Crystal Maiden".to_string(),
            &new_patch_name,
            ChangeData::OtherChange("Auto-attacks now take priority when determining kill credit".to_string()));
        let mut new_patch = vec![new_change_1, new_change_2, new_change_3, new_change_4];

        old_patch.append(&mut new_patch);
        let result = patch_diff(old_patch);

        assert_eq!(PatchChange::new(
            &"Items > Blade Mail > Duration".to_string(),
            &"7.32a".to_string(),
            ChangeData::AbsoluteChange("4.5s".to_string(), "6.5s".to_string())), result[5])
    }

    #[test]
    fn abs_num_parse_works() {
        let change_line = "Duration increased from 4.5s to 5.5s";
        let tree_location = "Items > Blade Mail".to_string();
        let version = "7.32";
        let result = PatchChange::parse_text(change_line, tree_location, version);
        assert_eq!(PatchChange::new(
            &"Items > Blade Mail > Duration".to_string(),
            &"7.32".to_string(),
            ChangeData::AbsoluteChange("4.5s".to_string(), "5.5s".to_string())
        ), result)
    }

    #[test]
    fn rel_num_parse_works() {
        let change_line = "Base armor increased by 1";
        let tree_location = "Heroes > Zeus".to_string();
        let version = "7.32";
        let result = PatchChange::parse_text(change_line, tree_location, version);
        assert_eq!(PatchChange::new(
            &"Heroes > Zeus > Base armor".to_string(),
            &"7.32".to_string(),
            ChangeData::RelativeChange(1)
        ), result)
    }

    #[test]
    fn abs_txt_parse_works() {
        let change_line = "Level 10 Talent OP replaced with Why would anyone take this?";
        let tree_location = "Heroes > Dark Willow > Talent".to_string();
        let version = "7.32";
        let result = PatchChange::parse_text(change_line, tree_location, version);
        assert_eq!(PatchChange::new(
            &"Heroes > Dark Willow > Talent > Level 10 Talent".to_string(),
            &"7.32".to_string(),
            ChangeData::AbsoluteChange("OP".to_string(), "Why would anyone take this?".to_string())
        ), result)
    }

    #[test]
    fn new_num_parse_works() {
        let change_line = "Avatar now has a 50 Mana Cost";
        let tree_location = "Items > Black King Bar".to_string();
        let version = "7.32";
        let result = PatchChange::parse_text(change_line, tree_location, version);
        assert_eq!(PatchChange::new(
            &"Items > Black King Bar > Mana Cost".to_string(),
            &"7.32".to_string(),
            ChangeData::AbsoluteChange("0".to_string(), "50".to_string())
        ), result)
    }

    #[test]
    fn other_parse_works() {
        let change_line = "Random Change";
        let tree_location = "Heroes > Crystal Maiden".to_string();
        let version = "7.32";
        let result = PatchChange::parse_text(change_line, tree_location, version);
        assert_eq!(PatchChange::new(
            &"Heroes > Crystal Maiden".to_string(),
            &"7.32".to_string(),
            ChangeData::OtherChange("Random Change".to_string())
        ), result)
    }

    #[test]
    fn abs_change_write_works() {
        let change = PatchChange::new(
            &"Items > Blade Mail > Duration".to_string(),
            &"7.32".to_string(),
            ChangeData::AbsoluteChange("4.5s".to_string(), "5.5s".to_string())
        );
        let result = change.write_text();
        assert_eq!("Items > Blade Mail > Duration increased from 4.5s to 5.5s".to_string(), result)
    }

    #[test]
    fn rel_change_write_works() {
        let change = PatchChange::new(
            &"Heroes > Zeus > Base armor".to_string(),
            &"7.32".to_string(),
            ChangeData::RelativeChange(1)
        );
        let result = change.write_text();
        assert_eq!("Heroes > Zeus > Base armor increased by 1".to_string(), result)
    }

    #[test]
    fn other_change_write_works() {
        let change = PatchChange::new(
            &"Heroes > Zeus".to_string(),
            &"7.32".to_string(),
            ChangeData::OtherChange("Random Change".to_string())
        );
        let result = change.write_text();
        assert_eq!("Heroes > Zeus > Random Change".to_string(), result)
    }

    #[test]
    fn abs_change_inc_works() {
        let old = "1s".to_string();
        let new = "2s".to_string();
        assert_eq!("increased".to_string(), absolute_change_direction(&old, &new));
    }

    #[test]
    fn abs_change_dec_works() {
        let old = "2s".to_string();
        let new = "1s".to_string();
        assert_eq!("decreased".to_string(), absolute_change_direction(&old, &new));
    }

    #[test]
    fn abs_change_res_works() {
        let old = "1/3/4/6s".to_string();
        let new = "2/3/4/5s".to_string();
        assert_eq!("rescaled".to_string(), absolute_change_direction(&old, &new));
    }
}
