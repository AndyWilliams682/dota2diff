use scraper::{Html, Selector, ElementRef};
use std::fs;

mod model;
pub use crate::model::{PatchChange, patch_diff};

fn get_version_list() -> Vec<String> {
    let paths = fs::read_dir("./html").unwrap();
    let mut version_file_list: Vec<String> = vec![];

    for path in paths {
        let version = path.unwrap().path().to_str().unwrap().to_string();
        version_file_list.push(version);
    }
    version_file_list
}

fn read_html_from_file(version: &str) -> Html {
    let file_path = format!("{}", version);
    let body = fs::read_to_string(file_path).unwrap();
    Html::parse_document(&body)
}

fn parse_patch_document(document: Html, version: &str) -> Vec<PatchChange> {
    let primary_div = Selector::parse(".mw-parser-output > *").unwrap();

    let mut current_h2 = "".to_string();
    let mut current_h3 = "".to_string();

    let mut patch_changes: Vec<PatchChange> = vec![];

    for element in document.select(&primary_div) {
        if element.value().name() == "h2" {
            current_h2 = element.text().next().unwrap().trim().to_string();
        } else if element.value().name() == "h3" {
            current_h3 = element.text().next().unwrap().trim().to_string();
        } else if element.value().name() == "ul" {
            let tree_loc = format!("{} > {}", current_h2, current_h3);
            if current_h2 == "General" || current_h2 == "Additional Content" {
                continue;
            }
            patch_changes.append(&mut parse_ul_element(element, tree_loc, version));
        }
    }
    patch_changes
}

fn parse_ul_element(ul: ElementRef, tree_loc: String, version: &str) -> Vec<PatchChange> {
    let mut ul_changes: Vec<PatchChange> = vec![];
    let b_selector = Selector::parse("b").unwrap();
    let mut b_values = ul.select(&b_selector);
    let mut next_b = b_values.next();
    let mut current_b = "".to_string();

    let change_lines = ul.text();
    for mut change_line in change_lines {
        change_line = change_line.trim();
        if change_line == "" {
            continue
        }

        change_line = change_line.split(" (").next().unwrap();

        if next_b != None {
            if change_line == next_b.unwrap().text().next().unwrap() {
                current_b = change_line.to_string();
                next_b = b_values.next();
                continue;
            }
        }

        let mut tree_location = tree_loc.to_string();
        if current_b != "".to_string() {
            tree_location.push_str(&format!(" > {}", current_b));
        }

        let parsed_text = PatchChange::parse_text(change_line, tree_location, &version);
        ul_changes.push(parsed_text);
    }

    ul_changes
}

fn get_diff_between(a: &str, b: &str) -> Vec<PatchChange> {
    let mut old_path = format!("./html/{}.html", a);
    let mut new_path = format!("./html/{}.html", b);

    if a > b {
        old_path = format!("./html/{}.html", b);
        new_path = format!("./html/{}.html", a);
    }
    let version_list = get_version_list();
    let mut gathering_patches = false;

    let mut combined_patches: Vec<PatchChange> = vec![];

    for version in version_list {
        if !gathering_patches && version == old_path {
            gathering_patches = true
        }
        if gathering_patches {
            if version == new_path {
                gathering_patches = false
            }
            let document = read_html_from_file(&version);
            combined_patches.append(&mut parse_patch_document(document, &version))
        }
    }
    patch_diff(combined_patches)
}

fn save_diff_as_html(diff_result: Vec<PatchChange>) {
    let mut result = "<div>".to_string();

    let mut current_h2 = "".to_string();
    let mut current_h3 = "".to_string();
    let mut current_b = "".to_string();

    for change in diff_result {
        let change_text = change.write_text();

        // Handling RelativeChangee values of 0 (no net change between patches)
        if change_text.contains("unchanged") {
            continue
        }

        let headers: Vec<&str> = change_text.split(" > ").collect();

        if headers[0] != current_h2 {
            if current_h2 == "".to_string() {
                result.push_str("<h2>");
            } else {
                if current_b != "".to_string() {
                    result.push_str("</ul></li>")
                }
                result.push_str("</ul><h2>")
            }
            result.push_str(&headers[0]);
            result.push_str("</h2>");
            current_h2 = headers[0].to_string();
            current_h3 = "".to_string();
            current_b = "".to_string();
        }
        if headers[1] != current_h3 {
            if current_h3 == "".to_string() {
                result.push_str("<h3>");
            } else {
                if current_b != "".to_string() {
                    result.push_str("</ul></li>")
                }
                result.push_str("</ul><h3>");
            }
            result.push_str(&headers[1]);
            result.push_str("</h3><ul>");
            current_h3 = headers[1].to_string();
            current_b = "".to_string();
        }
        if headers.len() == 4 {
            if headers[2] != current_b {
                if current_b == "".to_string() {
                    result.push_str("<li>");
                } else {
                    result.push_str("</ul></li><li>");
                }
                result.push_str(&headers[2]);
                result.push_str("<ul>");
                current_b = headers[2].to_string();
            }
        }

        result.push_str(&format!("<li>{}</li>", headers[headers.len() - 1]));
    }

    if current_b == "".to_string() {
        result.push_str("</ul></div>");
    } else {
        result.push_str("</ul></li></ul></div>");
    }
    fs::write("./html/patch_diff.html", result).expect("Unable to write file");
}

fn main() {
    save_diff_as_html(get_diff_between("7.32", "7.32c"))
}
