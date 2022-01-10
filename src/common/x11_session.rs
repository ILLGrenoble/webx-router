
pub struct X11Session {
    username: String,
    display_id: String,
    xauthority_file_path: String,
}

impl X11Session {

    pub fn new(username: String, display_id: String, xauthority_file_path: String) -> Self {
        Self {
            username,
            display_id,
            xauthority_file_path
        }
    }

    pub fn username(&self) -> &str {
        return &self.username;
    }

    pub fn display_id(&self) -> &str {
        return &self.display_id;
    }

    pub fn xauthority_file_path(&self) -> &str {
        return &self.xauthority_file_path;
    }
}

