use mac_notification_sys::NotificationResponse;

pub fn handle_response(response: NotificationResponse) {
    match response {
        NotificationResponse::ActionButton(action_name) => {}
        NotificationResponse::Click => {}
        NotificationResponse::CloseButton(close_name) => {}
        NotificationResponse::Reply(response) => {}
        NotificationResponse::None => {}
    }
}
