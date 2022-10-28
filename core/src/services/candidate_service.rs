use chrono::Duration;
use entity::candidate;
use sea_orm::{DatabaseConnection, prelude::Uuid, ModelTrait};

use crate::{crypto::{self, hash_sha256}, Query, token::{generate_candidate_token, candidate_token::CandidateToken}, error::{ServiceError, USER_NOT_FOUND_ERROR, INVALID_CREDENTIALS_ERROR, DB_ERROR, USER_NOT_FOUND_BY_JWT_ID, USER_NOT_FOUND_BY_SESSION_ID}, Mutation};

pub struct CandidateService;

impl CandidateService {
    #[deprecated(note = "Use login instead")]
    pub async fn login(db: &DatabaseConnection, id: i32, password: String) -> Result<String, ServiceError> {
        let candidate = match Query::find_candidate_by_id(db, id).await {
            Ok(candidate) => match candidate {
                Some(candidate) => candidate,
                None => return Err(USER_NOT_FOUND_ERROR)
            },
            Err(_) => {return Err(DB_ERROR)}
        };
    
        
        let valid = crypto::verify_password(&password,&candidate.code )
            .expect("Invalid password");
        
        if !valid {
            return Err(INVALID_CREDENTIALS_ERROR)
        }

        let jwt = generate_candidate_token(candidate); // TODO better error handling
        Ok(jwt)
    }

    pub async fn get_session(db: &DatabaseConnection, user_id: i32, password: String) -> Result<String, ServiceError> {
        let candidate = match Query::find_candidate_by_id(db, user_id).await {
            Ok(candidate) => match candidate {
                Some(candidate) => candidate,
                None => return Err(USER_NOT_FOUND_ERROR)
            },
            Err(_) => {return Err(DB_ERROR)}
        };

        // compare passwords
        match crypto::verify_password(&password, &candidate.code) {
            Ok(valid) => {
                if !valid {
                    return Err(INVALID_CREDENTIALS_ERROR)
                }
            },
            Err(_) => {return Err(INVALID_CREDENTIALS_ERROR)}
        }

        // TODO delete old sessions?
    
        // user is authenticated, generate a session
        let random_uuid: Uuid = Uuid::new_v4();

        let jwt = generate_candidate_token(candidate);

        let session = match Mutation::insert_session(db, user_id, random_uuid, hash_sha256(jwt)).await {
            Ok(session) => session,
            Err(_) => return Err(DB_ERROR)
        };

        Ok(session.id.to_string())
    }

    pub async fn authenticate_candidate(db: &DatabaseConnection, token: CandidateToken) -> Result<candidate::Model, ServiceError> {
        let candidate = match Query::find_candidate_by_id(db, token.application_id).await {
            Ok(candidate) => match candidate {
                Some(candidate) => candidate,
                None => return Err(USER_NOT_FOUND_BY_JWT_ID)
            },
            Err(_) => {return Err(DB_ERROR)}
        };

        Ok(candidate)
    } 

    pub async fn auth_user_session(db: &DatabaseConnection, uuid: Uuid) -> Result<candidate::Model, ServiceError> {
        let session = match Query::find_session_by_uuid(db, uuid).await {
            Ok(session) => match session {
                Some(session) => session,
                None => return Err(USER_NOT_FOUND_BY_SESSION_ID)
            },
            Err(_) => {return Err(DB_ERROR)}
        };

        let limit = session.created_at.checked_add_signed(Duration::days(1)).unwrap();
        let now = chrono::Utc::now().naive_utc();
        // check if session is expired
        if now > limit {
            // delete session
            Mutation::delete_session(db, session.id).await.unwrap();
            return Err(USER_NOT_FOUND_BY_SESSION_ID)
        }

        let candidate = match session.find_related(candidate::Entity).one(db).await {
            Ok(candidate) => match candidate {
                Some(candidate) => candidate,
                None => return Err(USER_NOT_FOUND_BY_JWT_ID)
            },
            Err(_) => {return Err(DB_ERROR)}
        };

        Ok(candidate)
    }
}



#[cfg(test)]
mod tests {
    use entity::candidate;
    use sea_orm::{DbConn, Database, sea_query::TableCreateStatement, DbBackend, Schema, ConnectionTrait};
    use serde_json::json;

    use crate::{crypto, Mutation, services::candidate_service::CandidateService, token};

    #[cfg(test)]
    async fn get_memory_sqlite_connection() -> DbConn {
        let base_url = "sqlite::memory:";
        let db: DbConn = Database::connect(base_url).await.unwrap();
    
        let schema = Schema::new(DbBackend::Sqlite);
        let stmt: TableCreateStatement = schema.create_table_from_entity(candidate::Entity);
        db.execute(db.get_database_backend().build(&stmt)).await.unwrap();
        db
    }
    
    #[tokio::test]
    async fn test_create_candidate() {
        let db = get_memory_sqlite_connection().await;
    
        let form = serde_json::from_value(json!({
                "application": 5555555,
            })).unwrap();
    
        let candidate = Mutation::create_candidate(&db, form, &"Tajny_kod".to_string()).await.unwrap();
    
        assert_eq!(candidate.application, 5555555);
        assert_ne!(candidate.code, "Tajny_kod".to_string());
        assert!(crypto::verify_password("Tajny_kod", &*candidate.code).ok().unwrap());
    }
    
    
    #[tokio::test]
    async fn test_candidate_jwt() {
        let db = &get_memory_sqlite_connection().await;
    
        let form = serde_json::from_value(json!({
            "application": 5555555,
        })).unwrap();
    
        let candidate = Mutation::create_candidate(&db, form, &"Tajny_kod".to_string()).await.unwrap();
    
        let jwt = CandidateService::login(db, 5555555, "Tajny_kod".to_string()).await.ok().unwrap();
    
        let claims = token::decode_candidate_token(jwt).ok().unwrap().claims;
    
        assert_eq!(claims.application_id, candidate.application);
    }
}