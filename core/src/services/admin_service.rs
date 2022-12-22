use async_trait::async_trait;
use entity::{admin, admin_session};
use sea_orm::{prelude::Uuid, DbConn, IntoActiveModel};

use crate::{crypto, error::ServiceError, Query, Mutation, models::auth::AuthenticableTrait};

use super::session_service::SessionService;

pub struct AdminService;

impl AdminService {
    async fn decrypt_private_key(
        db: &DbConn,
        admin_id: i32,
        password: String,
    ) -> Result<String, ServiceError> {
        let admin = Query::find_admin_by_id(db, admin_id).await?.ok_or(ServiceError::InvalidCredentials)?;
        let private_key_encrypted = admin.private_key;
        let private_key = crypto::decrypt_password(private_key_encrypted, password).await?;

        Ok(private_key)
    }
}

#[async_trait]
impl AuthenticableTrait for AdminService {
    type User = admin::Model;
    type Session = admin_session::Model;

    async fn login(
        db: &DbConn,
        admin_id: i32,
        password: String,
        ip_addr: String,
    ) -> Result<(String, String), ServiceError> {
        let admin = Query::find_admin_by_id(db, admin_id).await?.ok_or(ServiceError::InvalidCredentials)?;

        let session_id = Self::new_session(db,
            admin.clone(),
            password.clone(),
            ip_addr
        )
            .await?;
        
        let private_key = Self::decrypt_private_key(db, admin.id, password).await?;
        Ok((session_id, private_key))
    }

    async fn auth(db: &DbConn, session_uuid: Uuid) -> Result<admin::Model, ServiceError> {
        let session = Query::find_admin_session_by_uuid(db, session_uuid)
            .await?
            .ok_or(ServiceError::Unauthorized)?;

        if !SessionService::is_valid(&session).await? {
            Mutation::delete_session(db, session.into_active_model()).await?;
            return Err(ServiceError::ExpiredSession);
        }

        let admin = Query::find_admin_by_id(db, session.admin_id.unwrap())
            .await?
            .ok_or(ServiceError::CandidateNotFound)?;

        Ok(admin)
    }

    async fn logout(db: &DbConn, session: admin_session::Model) -> Result<(), ServiceError> {
        Mutation::delete_session(db, session.into_active_model()).await?;
        Ok(())
    }

    async fn new_session(
        db: &DbConn,
        admin: admin::Model,
        password: String,
        ip_addr: String,
    ) -> Result<String, ServiceError> {
        if !crypto::verify_password(password.clone(), admin.password.clone()).await? {
            return Err(ServiceError::InvalidCredentials);
        }
        // user is authenticated, generate a new session
        let random_uuid: Uuid = Uuid::new_v4();

        let session = Mutation::insert_admin_session(db, admin.id, random_uuid, ip_addr).await?;

        Self::delete_old_sessions(db, admin, 1).await?;

        Ok(session.id.to_string())
    }
    async fn delete_old_sessions(
        db: &DbConn,
        admin: admin::Model,
        keep_n_recent: usize,
    ) -> Result<(), ServiceError> {
        let sessions = Query::find_related_admin_sessions(db, admin)
            .await?
            .iter()
            .map(|s| s.clone().into_active_model())
            .collect();

        SessionService::delete_sessions(db, sessions, keep_n_recent).await?;
        Ok(())
    }

}

#[cfg(test)]
mod admin_tests {
    use chrono::Local;
    use entity::admin;
    use sea_orm::{Set, ActiveModelTrait};

    use crate::{utils::db::get_memory_sqlite_connection, error::ServiceError};

    use super::*;


    #[tokio::test]
    async fn test_admin_login() -> Result<(), ServiceError> {
        let db = get_memory_sqlite_connection().await;
        let admin = admin::ActiveModel {
            id: Set(1),
            name: Set("Admin".to_owned()),
            public_key: Set("age1u889gp407hsz309wn09kxx9anl6uns30m27lfwnctfyq9tq4qpus8tzmq5".to_owned()),
            // AGE-SECRET-KEY-14QG24502DMUUQDT2SPMX2YXPSES0X8UD6NT0PCTDAT6RH8V5Q3GQGSRXPS
            private_key: Set("5KCEGk0ueWVGnu5Xo3rmpLoilcVZ2ZWmwIcdZEJ8rrBNW7jwzZU/XTcTXtk/xyy/zjF8s+YnuVpOklQvX3EC/Sn+ZwyPY3jokM2RNwnZZlnqdehOEV1SMm/Y".to_owned()),
            // test
            password: Set("$argon2i$v=19$m=6000,t=3,p=10$WE9xCQmmWdBK82R4SEjoqA$TZSc6PuLd4aWK2x2WAb+Lm9sLySqjK3KLbNyqyQmzPQ".to_owned()),
            created_at: Set(Local::now().naive_local()),
            updated_at: Set(Local::now().naive_local()),
            ..Default::default()
        }
            .insert(&db)
            .await?;

        let (session_id, _private_key) = AdminService::login(&db, admin.id, "test".to_owned(), "127.0.0.1".to_owned()).await?;

        let logged_admin = AdminService::auth(&db, session_id.parse().unwrap()).await?;

        assert_eq!(logged_admin.id, 1);
        assert_eq!(logged_admin.name, "Admin");
        

        Ok(())

    }
}