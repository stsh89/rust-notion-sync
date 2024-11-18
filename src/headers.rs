use ureq::Request;

pub trait SetAuthorizationHeader {
    fn set_authorization_header(self, api_key: &str) -> Request;
}

pub trait SetDefaultHeaders {
    fn set_default_headers(self) -> Request;
}

impl SetAuthorizationHeader for Request {
    fn set_authorization_header(self, api_key: &str) -> Request {
        self.set("Authorization", &format!("Bearer {}", api_key))
    }
}

impl SetDefaultHeaders for Request {
    fn set_default_headers(self) -> Request {
        self.set("Content-Type", "application/json")
            .set("Notion-Version", "2022-06-28")
    }
}
