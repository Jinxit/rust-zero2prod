table! {
    subscriptions (id) {
        id -> Uuid,
        email -> Text,
        name -> Text,
        subscribed_at -> Timestamptz,
        status -> Text,
    }
}

table! {
    subscription_tokens (subscription_token) {
        subscription_token -> Text,
        subscriber_id -> Uuid,
    }
}
