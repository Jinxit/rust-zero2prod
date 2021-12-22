use rocket::form::Form;

#[derive(FromForm)]
pub struct FormData {
    name: String,
    email: String,
}

#[post("/subscriptions", data = "<form>")]
pub async fn subscribe(form: Form<FormData>) {}