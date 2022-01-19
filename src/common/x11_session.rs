
pub struct X11Session {
    session_id: String,
    username: String,
    display_id: String,
    xauthority_file_path: String,
}

impl X11Session {

    pub fn new(session_id: String, username: String, display_id: String, xauthority_file_path: String) -> Self {
        Self {
            session_id,
            username,
            display_id,
            xauthority_file_path
        }
    }

    pub fn session_id(&self) -> &str {
        return &self.session_id;
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

