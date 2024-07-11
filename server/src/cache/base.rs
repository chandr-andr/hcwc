trait CacheDB {
    async fn startup(&mut self);

    async fn shutdown(&mut self);

    async fn add_server(&mut self, server_uuid: &str);

    async fn remove_server(&mut self, server_uuid: &str);

    async fn add_new_user(&mut self, user_id: &str);

    async fn remove_user(&mut self, user_id: &str);

    async fn is_user_exist(&mut self, user_id: &str);
}
