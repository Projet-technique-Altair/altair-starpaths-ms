use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::starpath::{Starpath, StarpathRow},
    models::starpath_input::{AddStarpathLabInput, CreateStarpathInput, UpdateStarpathInput},
    models::starpath_lab::{StarpathLab, StarpathLabRow},
    models::starpath_progress::{StarpathProgress, StarpathProgressRow},
};

#[derive(Clone)]
pub struct StarpathsService {
    db: PgPool,
}

impl StarpathsService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    // =========================
    // GET /starpaths
    // =========================
    pub async fn list_starpaths(&self) -> Result<Vec<Starpath>, AppError> {
        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT *
            FROM starpaths
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Starpath::from).collect())
    }

    // =========================
    // GET /starpaths/:id
    // =========================
    pub async fn get_starpath(&self, starpath_id: Uuid) -> Result<Option<Starpath>, AppError> {
        let row = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT *
            FROM starpaths
            WHERE starpath_id = $1
            "#,
        )
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row.map(Starpath::from))
    }

    // ==========================
    // GET /mystarpaths (creator's starpaths only)
    // ==========================
    pub async fn my_starpaths(&self, creator_id: Uuid) -> Result<Vec<Starpath>, AppError> {

        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                starpath_id,
                creator_id,
                name,
                description,
                difficulty,
                created_at
            FROM starpaths
            WHERE creator_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(creator_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Starpath::from).collect())
    }

    // ==========================
    // SEARCH STARPATHS
    // ==========================
    pub async fn search_starpaths(
        &self,
        query: String,
    ) -> Result<Vec<Starpath>, AppError> {

        let pattern = format!("%{}%", query);

        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                starpath_id,
                creator_id,
                name,
                description,
                difficulty,
                created_at
            FROM starpaths
            WHERE name ILIKE $1
            ORDER BY name
            LIMIT 10
            "#,
        )
        .bind(pattern)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // =========================
    // POST /starpaths
    // =========================
    pub async fn create_starpath(
        &self,
        creator_id: Uuid,
        name: String,
        description: Option<String>,
        difficulty: Option<String>,
    ) -> Result<Starpath, AppError>{
        let row = sqlx::query_as::<_, StarpathRow>(
            r#"
            INSERT INTO starpaths (
                creator_id,
                name,
                description,
                difficulty
            )
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(creator_id)
        .bind(name)
        .bind(description)
        .bind(difficulty)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(Starpath::from(row))
    }

    // =========================
    // PUT /starpaths/:id
    // =========================
    pub async fn update_starpath(
        &self,
        starpath_id: Uuid,
        input: UpdateStarpathInput,
    ) -> Result<Option<Starpath>, AppError> {
        let row = sqlx::query_as::<_, StarpathRow>(
            r#"
            UPDATE starpaths
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                difficulty = COALESCE($4, difficulty)
            WHERE starpath_id = $1
            RETURNING *
            "#,
        )
        .bind(starpath_id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.difficulty)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row.map(Starpath::from))
    }

    // =========================
    // DELETE /starpaths/:id
    // =========================
    pub async fn delete_starpath(&self, starpath_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM starpaths
            WHERE starpath_id = $1
            "#,
        )
        .bind(starpath_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(result.rows_affected())
    }

    pub async fn get_starpath_labs(&self, starpath_id: Uuid) -> Result<Vec<StarpathLab>, AppError> {
        let rows = sqlx::query_as::<_, StarpathLabRow>(
            r#"
            SELECT starpath_id, lab_id, position
            FROM starpath_labs
            WHERE starpath_id = $1
            ORDER BY position ASC
            "#,
        )
        .bind(starpath_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(StarpathLab::from).collect())
    }

    pub async fn add_lab_to_starpath(
        &self,
        starpath_id: Uuid,
        input: AddStarpathLabInput,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO starpath_labs (starpath_id, lab_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(starpath_id)
        .bind(input.lab_id)
        .bind(input.position)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Conflict(e.to_string()))?;

        Ok(())
    }

    pub async fn update_starpath_lab_position(
        &self,
        starpath_id: Uuid,
        lab_id: Uuid,
        position: i32,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE starpath_labs
            SET position = $3
            WHERE starpath_id = $1 AND lab_id = $2
            "#,
        )
        .bind(starpath_id)
        .bind(lab_id)
        .bind(position)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Lab not found in starpath".into()));
        }

        Ok(())
    }

    pub async fn remove_lab_from_starpath(
        &self,
        starpath_id: Uuid,
        lab_id: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM starpath_labs
            WHERE starpath_id = $1 AND lab_id = $2
            "#,
        )
        .bind(starpath_id)
        .bind(lab_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Lab not found in starpath".into()));
        }

        Ok(())
    }

    pub async fn start_starpath(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
    ) -> Result<StarpathProgress, AppError> {
        // 1️⃣ Vérifier si déjà commencé (idempotence)
        if let Some(row) = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT *
            FROM user_starpath_progress
            WHERE user_id = $1 AND starpath_id = $2
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        {
            return Ok(StarpathProgress::from(row));
        }

        // 2️⃣ Créer progression
        let row = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            INSERT INTO user_starpath_progress (
                user_id,
                starpath_id,
                current_position,
                status
            )
            VALUES ($1, $2, 0, 'in_progress')
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(StarpathProgress::from(row))
    }

    // =========================
    // GET user starpath progress
    // =========================
    pub async fn get_starpath_progress(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
    ) -> Result<Option<StarpathProgress>, AppError> {
        let row = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT
                user_id,
                starpath_id,
                current_position,
                status,
                started_at,
                completed_at
            FROM user_starpath_progress
            WHERE user_id = $1
              AND starpath_id = $2
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row.map(StarpathProgress::from))
    }
}
