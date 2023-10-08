use uuid::Uuid;

pub const ADMIN: Uuid = {
    let uid = Uuid::try_parse("22129c89-7069-49ce-9f4a-f85004a7f230");
    match uid {
        Ok(uid) => uid,
        Err(_) => {
            panic!("invalid uuid!")
        }
    }
};
