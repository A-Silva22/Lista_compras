use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{mysql::MySqlPool, FromRow};

#[derive(Debug, Clone, FromRow)]
pub struct AuthUser {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub email: String,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: i8,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Lista {
    pub id: i32,
    pub nome: String,
    pub dono_id: i32,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Artigo {
    pub id: i32,
    pub lista_id: Option<i32>,
    pub nome: String,
    pub quantidade: String,
    pub comprar: i8,
    pub criado_em: DateTime<Utc>,
    pub movido_em: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PartilhaUser {
    pub partilha_id: i32,
    pub utilizador_id: i32,
    pub username: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct LinkPartilha {
    pub id: i32,
    pub lista_id: i32,
    pub token: String,
    pub criado_em: DateTime<Utc>,
    pub expira_em: DateTime<Utc>,
    pub pode_adicionar: i8,
    pub pode_editar: i8,
    pub pode_apagar: i8,
    pub pode_toggle: i8,
}

#[derive(Clone)]
pub struct Db {
    pool: MySqlPool,
}

impl Db {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn user_by_username(&self, username: &str) -> sqlx::Result<Option<AuthUser>> {
        sqlx::query_as::<_, AuthUser>(
            "SELECT id, username, password, email, last_login, is_active
             FROM auth_user WHERE username = ? LIMIT 1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn user_by_email(&self, email: &str) -> sqlx::Result<Option<AuthUser>> {
        sqlx::query_as::<_, AuthUser>(
            "SELECT id, username, password, email, last_login, is_active
             FROM auth_user WHERE email = ? LIMIT 1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn user_by_id(&self, id: i32) -> sqlx::Result<Option<AuthUser>> {
        sqlx::query_as::<_, AuthUser>(
            "SELECT id, username, password, email, last_login, is_active
             FROM auth_user WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn username_taken(&self, username: &str) -> sqlx::Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auth_user WHERE username = ?",
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 > 0)
    }

    pub async fn email_taken_by_other(&self, email: &str, exclude_id: i32) -> sqlx::Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auth_user WHERE email = ? AND id <> ?",
        )
        .bind(email)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 > 0)
    }

    pub async fn create_user(&self, username: &str, password_hash: &str) -> sqlx::Result<i32> {
        // Mirror Django auth_user defaults.
        let now = Utc::now();
        let result = sqlx::query(
            "INSERT INTO auth_user
                (password, last_login, is_superuser, username, first_name, last_name, email,
                 is_staff, is_active, date_joined)
             VALUES (?, NULL, 0, ?, '', '', '', 0, 1, ?)",
        )
        .bind(password_hash)
        .bind(username)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_id() as i32)
    }

    pub async fn update_password(&self, user_id: i32, password_hash: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE auth_user SET password = ? WHERE id = ?")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Store a single-use password-reset token for a user.
    pub async fn create_password_reset(
        &self,
        user_id: i32,
        token: &str,
        expira_em: DateTime<Utc>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO compras_passwordreset (user_id, token, expira_em, usado, criado_em)
             VALUES (?, ?, ?, 0, ?)",
        )
        .bind(user_id)
        .bind(token)
        .bind(expira_em)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Check a reset token without consuming it (valid = unused + not expired).
    pub async fn password_reset_valid(&self, token: &str) -> sqlx::Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM compras_passwordreset
             WHERE token = ? AND usado = 0 AND expira_em > ? LIMIT 1",
        )
        .bind(token)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    /// Redeem a reset token: if valid (unused + not expired), mark it used and
    /// return the user id. Returns None if invalid/expired/already used.
    pub async fn consume_password_reset(&self, token: &str) -> sqlx::Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT user_id FROM compras_passwordreset
             WHERE token = ? AND usado = 0 AND expira_em > ?
             LIMIT 1",
        )
        .bind(token)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?;
        let Some((user_id,)) = row else { return Ok(None) };
        sqlx::query("UPDATE compras_passwordreset SET usado = 1 WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(Some(user_id))
    }

    pub async fn update_email(&self, user_id: i32, email: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE auth_user SET email = ? WHERE id = ?")
            .bind(email)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn touch_last_login(&self, user_id: i32) -> sqlx::Result<()> {
        sqlx::query("UPDATE auth_user SET last_login = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_default_lista(&self, owner_id: i32) -> sqlx::Result<()> {
        // Match the Django `Lista` model: nome, dono_id, criado_em.
        sqlx::query(
            "INSERT INTO compras_lista (nome, dono_id, criado_em)
             VALUES ('Casa', ?, ?)",
        )
        .bind(owner_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Lista CRUD (only owned lists for now; share-link/share-with-user
    // ── port comes later) ───────────────────────────────────────────────

    pub async fn lists_for_user(&self, user_id: i32) -> sqlx::Result<Vec<Lista>> {
        // Owned + shared. DISTINCT in case a row appears via both paths.
        sqlx::query_as::<_, Lista>(
            "SELECT DISTINCT l.id, l.nome, l.dono_id, l.criado_em
             FROM compras_lista l
             LEFT JOIN compras_listapartilha p ON p.lista_id = l.id
             WHERE l.dono_id = ? OR p.utilizador_id = ?
             ORDER BY l.criado_em DESC",
        )
        .bind(user_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Returns the lista if the user owns it OR is a shared collaborator.
    pub async fn get_accessible_lista(
        &self,
        lista_id: i32,
        user_id: i32,
    ) -> sqlx::Result<Option<Lista>> {
        sqlx::query_as::<_, Lista>(
            "SELECT DISTINCT l.id, l.nome, l.dono_id, l.criado_em
             FROM compras_lista l
             LEFT JOIN compras_listapartilha p ON p.lista_id = l.id
             WHERE l.id = ? AND (l.dono_id = ? OR p.utilizador_id = ?)
             LIMIT 1",
        )
        .bind(lista_id)
        .bind(user_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// IDs of lists OWNED by `user_id` that are shared in some way — either
    /// shared with another user or carrying at least one non-expired link.
    /// Used to badge shared lists in the picker.
    pub async fn shared_owned_list_ids(
        &self,
        user_id: i32,
    ) -> sqlx::Result<std::collections::HashSet<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT DISTINCT l.id
             FROM compras_lista l
             LEFT JOIN compras_listapartilha p ON p.lista_id = l.id
             LEFT JOIN compras_linkpartilha lk ON lk.lista_id = l.id AND lk.expira_em > ?
             WHERE l.dono_id = ? AND (p.id IS NOT NULL OR lk.id IS NOT NULL)",
        )
        .bind(Utc::now())
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// IDs of lists owned by `user_id` that are shared with at least one user.
    pub async fn user_shared_owned_ids(&self, user_id: i32) -> sqlx::Result<std::collections::HashSet<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT DISTINCT l.id FROM compras_lista l
             JOIN compras_listapartilha p ON p.lista_id = l.id
             WHERE l.dono_id = ?",
        ).bind(user_id).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// IDs of lists owned by `user_id` that carry at least one non-expired link.
    pub async fn link_shared_owned_ids(&self, user_id: i32) -> sqlx::Result<std::collections::HashSet<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT DISTINCT l.id FROM compras_lista l
             JOIN compras_linkpartilha k ON k.lista_id = l.id AND k.expira_em > ?
             WHERE l.dono_id = ?",
        ).bind(Utc::now()).bind(user_id).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn get_owned_lista(&self, lista_id: i32, user_id: i32) -> sqlx::Result<Option<Lista>> {
        sqlx::query_as::<_, Lista>(
            "SELECT id, nome, dono_id, criado_em
             FROM compras_lista WHERE id = ? AND dono_id = ?",
        )
        .bind(lista_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_lista(&self, nome: &str, owner_id: i32) -> sqlx::Result<i32> {
        let result = sqlx::query(
            "INSERT INTO compras_lista (nome, dono_id, criado_em)
             VALUES (?, ?, ?)",
        )
        .bind(nome)
        .bind(owner_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_id() as i32)
    }

    pub async fn delete_lista(&self, lista_id: i32, user_id: i32) -> sqlx::Result<u64> {
        // Drop everything that has an FK to compras_lista before the row itself.
        // Django CASCADEs through the ORM; raw SQL doesn't, and older schemas
        // also lack ON DELETE CASCADE.
        sqlx::query("DELETE FROM compras_artigo WHERE lista_id = ?")
            .bind(lista_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM compras_listapartilha WHERE lista_id = ?")
            .bind(lista_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM compras_linkpartilha WHERE lista_id = ?")
            .bind(lista_id)
            .execute(&self.pool)
            .await?;
        let result = sqlx::query("DELETE FROM compras_lista WHERE id = ? AND dono_id = ?")
            .bind(lista_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    // ── Artigo CRUD ─────────────────────────────────────────────────────

    /// Returns (despensa, a_comprar) ordered by movido_em desc.
    pub async fn articles_for_list(
        &self,
        lista_id: i32,
    ) -> sqlx::Result<(Vec<Artigo>, Vec<Artigo>)> {
        let rows: Vec<Artigo> = sqlx::query_as(
            "SELECT id, lista_id, nome, quantidade, comprar, criado_em, movido_em
             FROM compras_artigo
             WHERE lista_id = ?
             ORDER BY movido_em DESC",
        )
        .bind(lista_id)
        .fetch_all(&self.pool)
        .await?;
        let mut despensa = Vec::new();
        let mut comprar = Vec::new();
        for row in rows {
            if row.comprar == 0 {
                despensa.push(row);
            } else {
                comprar.push(row);
            }
        }
        Ok((despensa, comprar))
    }

    /// Find an artigo with the given name (case-insensitive exact match)
    /// in a specific list. Used for duplicate-detection on the add bar.
    pub async fn match_artigo_in_list(
        &self,
        lista_id: i32,
        nome: &str,
    ) -> sqlx::Result<Option<Artigo>> {
        sqlx::query_as::<_, Artigo>(
            "SELECT id, lista_id, nome, quantidade, comprar, criado_em, movido_em
             FROM compras_artigo
             WHERE lista_id = ? AND LOWER(nome) = LOWER(?)
             LIMIT 1",
        )
        .bind(lista_id)
        .bind(nome)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn add_artigo(
        &self,
        lista_id: i32,
        nome: &str,
        quantidade: &str,
    ) -> sqlx::Result<i32> {
        let now = Utc::now();
        let result = sqlx::query(
            "INSERT INTO compras_artigo
                (lista_id, nome, quantidade, comprar, criado_em, movido_em)
             VALUES (?, ?, ?, 1, ?, ?)",
        )
        .bind(lista_id)
        .bind(nome)
        .bind(quantidade)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_id() as i32)
    }

    /// destino: Some("comprar") | Some("despensa") | None (=flip).
    /// Returns the new comprar value if the row exists.
    pub async fn toggle_artigo(
        &self,
        artigo_id: i32,
        lista_id: i32,
        destino: Option<&str>,
    ) -> sqlx::Result<Option<bool>> {
        let row: Option<(i8,)> = sqlx::query_as(
            "SELECT comprar FROM compras_artigo WHERE id = ? AND lista_id = ?",
        )
        .bind(artigo_id)
        .bind(lista_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((current,)) = row else { return Ok(None); };
        let new_val: i8 = match destino {
            Some("despensa") => 0,
            Some("comprar") => 1,
            _ => if current == 0 { 1 } else { 0 },
        };
        sqlx::query(
            "UPDATE compras_artigo SET comprar = ?, movido_em = ?
             WHERE id = ? AND lista_id = ?",
        )
        .bind(new_val)
        .bind(Utc::now())
        .bind(artigo_id)
        .bind(lista_id)
        .execute(&self.pool)
        .await?;
        Ok(Some(new_val != 0))
    }

    pub async fn edit_artigo(
        &self,
        artigo_id: i32,
        lista_id: i32,
        nome: &str,
        quantidade: &str,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE compras_artigo SET nome = ?, quantidade = ?
             WHERE id = ? AND lista_id = ?",
        )
        .bind(nome)
        .bind(quantidade)
        .bind(artigo_id)
        .bind(lista_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_artigo(&self, artigo_id: i32, lista_id: i32) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "DELETE FROM compras_artigo WHERE id = ? AND lista_id = ?",
        )
        .bind(artigo_id)
        .bind(lista_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn search_artigos_for_user(
        &self,
        user_id: i32,
        q: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<String>> {
        // Suggest distinct artigo names from any list the user owns
        // or has been shared on, matching the prefix.
        let pattern = format!("{}%", q);
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT a.nome
             FROM compras_artigo a
             JOIN compras_lista l ON a.lista_id = l.id
             LEFT JOIN compras_listapartilha p
               ON p.lista_id = l.id AND p.utilizador_id = ?
             WHERE a.nome LIKE ? AND (l.dono_id = ? OR p.id IS NOT NULL)
             ORDER BY a.nome
             LIMIT ?",
        )
        .bind(user_id)
        .bind(pattern)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    // ── Share with another user (ListaPartilha) ──────────────────────────

    pub async fn partilhas_for_lista(&self, lista_id: i32) -> sqlx::Result<Vec<PartilhaUser>> {
        sqlx::query_as::<_, PartilhaUser>(
            "SELECT p.id AS partilha_id, p.utilizador_id, u.username
             FROM compras_listapartilha p
             JOIN auth_user u ON u.id = p.utilizador_id
             WHERE p.lista_id = ?
             ORDER BY u.username",
        )
        .bind(lista_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn add_partilha(&self, lista_id: i32, utilizador_id: i32) -> sqlx::Result<bool> {
        // Returns true if a new row was inserted, false if it already existed.
        let exists: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM compras_listapartilha
             WHERE lista_id = ? AND utilizador_id = ?",
        )
        .bind(lista_id)
        .bind(utilizador_id)
        .fetch_one(&self.pool)
        .await?;
        if exists.0 > 0 { return Ok(false); }
        sqlx::query(
            "INSERT INTO compras_listapartilha (lista_id, utilizador_id, criado_em)
             VALUES (?, ?, ?)",
        )
        .bind(lista_id)
        .bind(utilizador_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(true)
    }

    pub async fn remove_partilha(&self, lista_id: i32, utilizador_id: i32) -> sqlx::Result<u64> {
        let r = sqlx::query(
            "DELETE FROM compras_listapartilha
             WHERE lista_id = ? AND utilizador_id = ?",
        )
        .bind(lista_id)
        .bind(utilizador_id)
        .execute(&self.pool)
        .await?;
        Ok(r.rows_affected())
    }

    // ── Public share links (LinkPartilha) ────────────────────────────────

    pub async fn create_link(
        &self,
        lista_id: i32,
        token: &str,
        expira_em: DateTime<Utc>,
        pode_adicionar: bool,
        pode_editar: bool,
        pode_apagar: bool,
        pode_toggle: bool,
    ) -> sqlx::Result<i32> {
        let r = sqlx::query(
            "INSERT INTO compras_linkpartilha
              (lista_id, token, criado_em, expira_em, pode_adicionar, pode_editar, pode_apagar, pode_toggle)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(lista_id)
        .bind(token)
        .bind(Utc::now())
        .bind(expira_em)
        .bind(pode_adicionar as i8)
        .bind(pode_editar as i8)
        .bind(pode_apagar as i8)
        .bind(pode_toggle as i8)
        .execute(&self.pool)
        .await?;
        Ok(r.last_insert_id() as i32)
    }

    pub async fn active_links_for_lista(&self, lista_id: i32) -> sqlx::Result<Vec<LinkPartilha>> {
        sqlx::query_as::<_, LinkPartilha>(
            "SELECT id, lista_id, token, criado_em, expira_em,
                    pode_adicionar, pode_editar, pode_apagar, pode_toggle
             FROM compras_linkpartilha
             WHERE lista_id = ? AND expira_em > ?
             ORDER BY criado_em DESC",
        )
        .bind(lista_id)
        .bind(Utc::now())
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_link(&self, link_id: i32, lista_id: i32) -> sqlx::Result<u64> {
        let r = sqlx::query(
            "DELETE FROM compras_linkpartilha WHERE id = ? AND lista_id = ?",
        )
        .bind(link_id)
        .bind(lista_id)
        .execute(&self.pool)
        .await?;
        Ok(r.rows_affected())
    }

    pub async fn link_by_token(&self, token: &str) -> sqlx::Result<Option<LinkPartilha>> {
        sqlx::query_as::<_, LinkPartilha>(
            "SELECT id, lista_id, token, criado_em, expira_em,
                    pode_adicionar, pode_editar, pode_apagar, pode_toggle
             FROM compras_linkpartilha
             WHERE token = ? LIMIT 1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn lista_by_id(&self, lista_id: i32) -> sqlx::Result<Option<Lista>> {
        sqlx::query_as::<_, Lista>(
            "SELECT id, nome, dono_id, criado_em FROM compras_lista WHERE id = ?",
        )
        .bind(lista_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// direcao: "mais" | "menos". Numeric prefix only; mirrors Django's regex.
    pub async fn update_quantidade(
        &self,
        artigo_id: i32,
        lista_id: i32,
        direcao: &str,
    ) -> sqlx::Result<()> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT quantidade FROM compras_artigo WHERE id = ? AND lista_id = ?",
        )
        .bind(artigo_id)
        .bind(lista_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((q,)) = row else { return Ok(()); };
        let prefix: String = q.chars().take_while(|c| c.is_ascii_digit()).collect();
        let mut n: i64 = prefix.parse().unwrap_or(1);
        match direcao {
            "mais" => n += 1,
            "menos" if n > 1 => n -= 1,
            _ => {}
        }
        let new_q = format!("{}", n);
        sqlx::query(
            "UPDATE compras_artigo SET quantidade = ?
             WHERE id = ? AND lista_id = ?",
        )
        .bind(new_q)
        .bind(artigo_id)
        .bind(lista_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
