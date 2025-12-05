use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct SettingObject {
    pub name: String,
    pub value: String,
}

pub struct Settings {
    verify_storage: bool,
    include_verify: Vec<String>,
    exclude_verify: Vec<String>,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            verify_storage: true,
            include_verify: vec![],
            exclude_verify: vec![],
        }
    }

    pub fn should_verify_storage(&self, contract_address: &String) -> bool {
        if self.verify_storage {
            if self.include_verify.is_empty() && self.exclude_verify.is_empty() {
                return true;
            }
            if self.include_verify.contains(contract_address) {
                return true;
            }
            if self.exclude_verify.contains(contract_address) {
                return false;
            }
        }
        false
    }

    pub fn update_settings(&mut self, settings: Vec<SettingObject>) {
        for setting in settings {
            match setting.name.as_str() {
                "verify_storage" => self.verify_storage = setting.value == "true",
                "add_to_include_verify" => self.include_verify.push(setting.value),
                "add_to_exclude_verify" => self.exclude_verify.push(setting.value),
                "remove_from_include_verify" => {
                    if let Some(pos) = self.include_verify.iter().position(|x| x == &setting.value) {
                        self.include_verify.remove(pos);
                    }
                },
                "remove_from_exclude_verify" => {
                    if let Some(pos) = self.exclude_verify.iter().position(|x| x == &setting.value) {
                        self.exclude_verify.remove(pos);
                    }
                },
                _ => (),
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}
