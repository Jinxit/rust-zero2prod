use rocket::http::Header;
use rocket::response::Responder;

#[catch(401)]
pub fn unauthorized_request_credentials() -> RequestBasicAuth {
    RequestBasicAuth::new()
}

struct RequestBasicAuthHeader;

impl<'h> From<RequestBasicAuthHeader> for Header<'h> {
    fn from(_: RequestBasicAuthHeader) -> Self {
        Header {
            name: "WWW-Authenticate".into(),
            value: r#"Basic realm="publish""#.into(),
        }
    }
}

#[derive(Responder)]
#[response(status = 401)]
pub struct RequestBasicAuth {
    inner: (),
    basic_auth: RequestBasicAuthHeader,
}

impl RequestBasicAuth {
    fn new() -> RequestBasicAuth {
        RequestBasicAuth {
            inner: (),
            basic_auth: RequestBasicAuthHeader,
        }
    }
}

impl Default for RequestBasicAuth {
    fn default() -> Self {
        Self::new()
    }
}
