use crate::db::Db;
use crate::error::Error;
use surrealdb::types::RecordId;

pub struct RbacRepo {
    pub db: Db,
}

impl RbacRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn assign_role(&self, user_id: &str, role_id: &str) -> Result<(), Error> {
        let _ = self
            .db
            .query("RELATE type::record('user', $user)->assigned_role->type::record('role', $role)")
            .bind(("user", user_id.to_string()))
            .bind(("role", role_id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn has_permission(
        &self,
        user_id: &str,
        permission_name: &str,
    ) -> Result<bool, Error> {
        let mut response = self.db
            .query(
                "SELECT id FROM type::record('user', $user)->assigned_role->role->has_permission->permission WHERE name = $permission",
            )
            .bind(("user", user_id.to_string()))
            .bind(("permission", permission_name.to_string()))
            .await?;

        let perms: Vec<RecordId> = response.take(0)?;
        Ok(!perms.is_empty())
    }
}
