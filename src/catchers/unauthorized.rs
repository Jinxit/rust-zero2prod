use rocket::http::Header;
use rocket::response::Responder;

#[catch(401)]
pub fn unauthorized_request_credentials() -> RequestBasicAuth {
    RequestBasicAuth::new()
}

struct RequestBasicAuthHeader;

impl<'h> Into<Header<'h>> for RequestBasicAuthHeader {
    fn into(self) -> Header<'h> {
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
    pub fn new() -> RequestBasicAuth {
        RequestBasicAuth {
            inner: (),
            basic_auth: RequestBasicAuthHeader,
        }
    }
}
