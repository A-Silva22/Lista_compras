use askama::Template;

#[derive(Template)]
#[template(path = "entrar.html")]
pub struct EntrarPage<'a> {
    pub erro: &'a str,
    pub next: &'a str,
}

#[derive(Template)]
#[template(path = "registar.html")]
pub struct RegistarPage<'a> {
    pub erro: &'a str,
    pub next: &'a str,
}

#[derive(Template)]
#[template(path = "adicionar_email.html")]
pub struct AdicionarEmailPage<'a> {
    pub erro: &'a str,
    pub username: &'a str,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexPage {
    pub username: String,
    pub email: String,
    pub lists: Vec<ListEntry>,
    /// 0 = no active list. Avoids Option/borrow gymnastics in the template.
    pub active_id: i32,
    pub active_nome: String,
    pub despensa: Vec<crate::db::Artigo>,
    pub a_comprar: Vec<crate::db::Artigo>,
    pub partilhas: Vec<crate::db::PartilhaUser>,
    pub links: Vec<LinkRow>,
}

/// A list as shown in the hamburger picker, with a flag marking whether it is
/// shared (owned-by-someone-else, shared with users, or has an active link).
pub struct ListEntry {
    pub id: i32,
    pub nome: String,
    pub shared: bool,
}

pub struct LinkRow {
    pub id: i32,
    pub url: String,
    pub expira_em_str: String,
    pub pode_adicionar: bool,
    pub pode_editar: bool,
    pub pode_apagar: bool,
    pub pode_toggle: bool,
}

#[derive(Template)]
#[template(path = "link_publico.html")]
pub struct LinkPublicPage {
    pub token: String,
    pub lista_nome: String,
    pub despensa: Vec<crate::db::Artigo>,
    pub a_comprar: Vec<crate::db::Artigo>,
    pub pode_adicionar: bool,
    pub pode_editar: bool,
    pub pode_apagar: bool,
    pub pode_toggle: bool,
    pub expira_em_str: String,
    pub already_logged_in: bool,
}

#[derive(Template)]
#[template(path = "recuperar.html")]
pub struct RecuperarPage<'a> {
    pub enviado: bool,
    pub erro: &'a str,
}
