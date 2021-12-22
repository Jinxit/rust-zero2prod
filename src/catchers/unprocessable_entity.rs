use rocket::response::status;
use rocket::response::status::BadRequest;
use rocket::Request;

#[catch(422)]
pub fn unprocessable_entity_to_bad_request(_req: &Request) -> BadRequest<()> {
    status::BadRequest::<()>(None)
}
