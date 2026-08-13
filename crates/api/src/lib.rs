use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::Utc;
use rclone_backup_core::{
    GlobalNotificationSettings, NotificationConfig, NotificationTargetKind, Plan, PlanInput,
    REDACTED, RunRecord,
};
use rclone_backup_runner::Runner;
use rclone_backup_store::Store;
use serde::{Deserialize, Serialize};
use serde_json::json;
use subtle::ConstantTimeEq;
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer, limit::RequestBodyLimitLayer,
    set_header::SetResponseHeaderLayer,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub runner: Runner,
    pub public_auth: Option<(String, String)>,
    pub site_name: String,
}

const FRONTEND_ROUTES: &[&str] = &["/", "/plans", "/accounts", "/notifications", "/history"];

pub fn router(state: AppState) -> Router {
    let protected = FRONTEND_ROUTES
        .iter()
        .fold(Router::new(), |router, path| router.route(path, get(index)))
        .route("/app.css", get(css))
        .route("/app.js", get(js))
        .route("/api/docs", get(api_docs))
        .route("/api/openapi.json", get(openapi))
        .route("/api/status", get(status))
        .route("/api/rclone/providers", get(rclone_providers))
        .route(
            "/api/rclone/remotes",
            get(rclone_remotes).post(create_rclone_remote),
        )
        .route(
            "/api/rclone/remotes/{name}",
            axum::routing::put(update_rclone_remote).delete(delete_rclone_remote),
        )
        .route(
            "/api/rclone/remotes/{name}/test",
            axum::routing::post(test_rclone_remote),
        )
        .route("/api/plans", get(list_plans).post(create_plan))
        .route("/api/plans/{id}", put(update_plan).delete(delete_plan))
        .route(
            "/api/plans/{id}/archive-password",
            post(reveal_archive_password),
        )
        .route("/api/plans/{id}/run", post(run_plan))
        .route(
            "/api/notifications",
            get(get_notifications).put(update_notifications),
        )
        .route("/api/notifications/test", post(test_notification))
        .route("/api/runs", get(list_runs))
        .fallback(not_found)
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
    Router::new()
        .route("/api/health", get(health))
        .merge(protected)
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(CompressionLayer::new())
        .layer(CatchPanicLayer::new())
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"),
        ))
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let html = render_index_html(&state.site_name);
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(html))
        .unwrap()
}
fn render_index_html(site_name: &str) -> String {
    include_str!("../../../web/index.html")
        .replace("{{SITE_NAME}}", &escape_html(site_name))
        .replace("{{ASSET_VERSION}}", env!("CARGO_PKG_VERSION"))
}
async fn css() -> impl IntoResponse {
    static_asset(
        include_str!("../../../web/app.css"),
        "text/css; charset=utf-8",
    )
}
async fn js() -> impl IntoResponse {
    static_asset(
        include_str!("../../../web/app.js"),
        "text/javascript; charset=utf-8",
    )
}
fn static_asset(content: &'static str, content_type: &'static str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(content))
        .unwrap()
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
    rclone_version: Option<String>,
    site_name: String,
    time: chrono::DateTime<Utc>,
}
async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        rclone_version: state.runner.rclone_version().map(str::to_owned),
        site_name: state.site_name,
        time: Utc::now(),
    })
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod asset_tests {
    use super::escape_html;

    #[test]
    fn site_name_is_html_escaped() {
        assert_eq!(
            escape_html("Backup <home> & \"cloud\""),
            "Backup &lt;home&gt; &amp; &quot;cloud&quot;"
        );
    }
}

#[derive(Serialize)]
struct Status {
    service: &'static str,
    rclone_ready: bool,
    authentication_enabled: bool,
    rclone_stats: serde_json::Value,
}
async fn status(State(state): State<AppState>) -> Json<Status> {
    Json(Status {
        service: "online",
        rclone_ready: state.runner.rclone_ready(),
        authentication_enabled: state.public_auth.is_some(),
        rclone_stats: state.runner.rclone_stats().await,
    })
}

async fn rclone_providers(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(state.runner.rclone_providers().await?))
}

async fn rclone_remotes(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(state.runner.rclone_remotes().await?))
}

#[derive(Deserialize)]
struct RemoteInput {
    name: String,
    #[serde(rename = "type")]
    provider_type: String,
    #[serde(default)]
    parameters: serde_json::Value,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    result: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RemoteUpdateInput {
    #[serde(default)]
    parameters: serde_json::Value,
}

impl RemoteInput {
    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty()
            || self.name.chars().count() > 80
            || self
                .name
                .chars()
                .any(|c| c.is_control() || matches!(c, ':' | '/' | '\\'))
        {
            return Err("rclone alias is invalid".into());
        }
        if self.provider_type.is_empty()
            || self.provider_type.chars().count() > 80
            || !self
                .provider_type
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        {
            return Err("provider type is invalid".into());
        }
        if !self.parameters.is_object() {
            return Err("parameters must be an object".into());
        }
        Ok(())
    }
}

async fn create_rclone_remote(
    State(state): State<AppState>,
    Json(input): Json<RemoteInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    input.validate().map_err(ApiError::validation)?;
    let response = if let Some(flow_state) = &input.state {
        state
            .runner
            .continue_rclone_remote(
                &input.name,
                &input.provider_type,
                input.parameters,
                flow_state,
                input.result.unwrap_or_else(|| json!({})),
            )
            .await?
    } else {
        state
            .runner
            .create_rclone_remote(&input.name, &input.provider_type, input.parameters)
            .await?
    };
    let needs_more_input = remote_needs_input(&response);
    if !needs_more_input {
        state.runner.test_rclone_remote(&input.name).await?;
    }
    Ok((StatusCode::CREATED, Json(response)))
}

fn remote_needs_input(response: &serde_json::Value) -> bool {
    response
        .get("State")
        .or_else(|| response.get("state"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|state| !state.is_empty())
        || response
            .get("Option")
            .or_else(|| response.get("option"))
            .is_some_and(|option| !option.is_null())
}

async fn delete_rclone_remote(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    validate_remote_name(&name)?;
    let references: Vec<String> = state
        .store
        .list_plans()
        .await?
        .into_iter()
        .filter(|plan| plan.remotes.iter().any(|remote| remote.name == name))
        .map(|plan| plan.name)
        .collect();
    if !references.is_empty() {
        return Err(ApiError::conflict(format!(
            "remote is referenced by backup plan(s): {}",
            references.join(", ")
        )));
    }
    state.runner.delete_rclone_remote(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_rclone_remote(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(input): Json<RemoteUpdateInput>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_remote_name(&name)?;
    if !input.parameters.is_object() {
        return Err(ApiError::validation("parameters must be an object".into()));
    }
    let response = state
        .runner
        .update_rclone_remote(&name, input.parameters)
        .await?;
    state.runner.test_rclone_remote(&name).await?;
    Ok(Json(response))
}

async fn ensure_plan_aliases(state: &AppState, input: &PlanInput) -> ApiResult<()> {
    let summaries = state.runner.rclone_remotes().await?;
    let aliases: std::collections::HashSet<&str> = summaries
        .get("remotes")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|remote| remote.get("name").and_then(serde_json::Value::as_str))
        .collect();
    let missing: Vec<&str> = input
        .remotes
        .iter()
        .map(|remote| remote.name.as_str())
        .filter(|name| !aliases.contains(name))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ApiError::validation(format!(
            "unknown rclone alias(es): {}",
            missing.join(", ")
        )))
    }
}

async fn test_rclone_remote(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_remote_name(&name)?;
    state.runner.test_rclone_remote(&name).await?;
    Ok(Json(json!({ "ok": true })))
}

fn validate_remote_name(name: &str) -> ApiResult<()> {
    let input = RemoteInput {
        name: name.to_owned(),
        provider_type: "placeholder".into(),
        parameters: json!({}),
        state: None,
        result: None,
    };
    input.validate().map_err(ApiError::validation)
}

async fn require_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    if matches!(
        *request.method(),
        axum::http::Method::POST | axum::http::Method::PUT | axum::http::Method::DELETE
    ) && !same_origin_or_non_browser(&headers)
    {
        return Err(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("cross-origin request rejected"))
            .unwrap());
    }
    let Some((expected_user, expected_password)) = &state.public_auth else {
        return Ok(next.run(request).await);
    };
    let valid = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
        .and_then(|value| STANDARD.decode(value).ok())
        .and_then(|value| String::from_utf8(value).ok())
        .and_then(|value| {
            value
                .split_once(':')
                .map(|(u, p)| (u.to_owned(), p.to_owned()))
        })
        .is_some_and(|(user, password)| {
            user.as_bytes().ct_eq(expected_user.as_bytes()).into()
                && password
                    .as_bytes()
                    .ct_eq(expected_password.as_bytes())
                    .into()
        });
    if valid {
        Ok(next.run(request).await)
    } else {
        Err(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(
                header::WWW_AUTHENTICATE,
                "Basic realm=\"Rclone Backup\", charset=\"UTF-8\"",
            )
            .body(Body::from("authentication required"))
            .unwrap())
    }
}

fn same_origin_or_non_browser(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    origin
        .to_str()
        .ok()
        .and_then(|value| url::Url::parse(value).ok())
        .is_some_and(|value| {
            let Some(host_name) = value.host_str() else {
                return false;
            };
            let origin_authority = match value.port() {
                Some(port) => format!("{host_name}:{port}"),
                None => host_name.to_owned(),
            };
            origin_authority.eq_ignore_ascii_case(host)
        })
}

async fn openapi() -> impl IntoResponse {
    Json(json!({
        "openapi": "3.1.0",
        "info": { "title": "Rclone Backup API", "version": env!("CARGO_PKG_VERSION") },
        "servers": [{ "url": "/" }],
        "paths": {
            "/api/health": { "get": { "summary": "Liveness", "responses": { "200": { "description": "Service is alive" }}}},
            "/api/status": { "get": { "summary": "Service, rclone readiness, and transfer status", "responses": { "200": { "description": "Current status" }}}},
            "/api/rclone/providers": { "get": { "summary": "List provider schemas from the bundled rclone", "responses": { "200": { "description": "Provider schemas" }}}},
            "/api/rclone/remotes": {
                "get": { "summary": "List configured rclone aliases", "responses": { "200": { "description": "Aliases" }}},
                "post": { "summary": "Create or continue a guided rclone remote configuration", "responses": { "201": { "description": "Created or next configuration question" }, "422": { "description": "Validation error" }}}
            },
            "/api/rclone/remotes/{name}": {
                "put": { "summary": "Update and test an rclone alias", "responses": { "200": { "description": "Updated and connected" }}},
                "delete": { "summary": "Delete an unreferenced rclone alias", "responses": { "204": { "description": "Deleted" }, "409": { "description": "Alias is referenced by a plan" }}}
            },
            "/api/rclone/remotes/{name}/test": { "post": { "summary": "Test an rclone alias", "responses": { "200": { "description": "Connection passed" }}}},
            "/api/plans": {
                "get": { "summary": "List backup plans", "responses": { "200": { "description": "Plans" }}},
                "post": { "summary": "Create a backup plan", "responses": { "201": { "description": "Created" }, "422": { "description": "Validation error" }}}
            },
            "/api/plans/{id}": {
                "put": { "summary": "Update a backup plan", "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" }}], "responses": { "200": { "description": "Updated" }}},
                "delete": { "summary": "Delete a backup plan", "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" }}], "responses": { "204": { "description": "Deleted" }}}
            },
            "/api/plans/{id}/run": { "post": { "summary": "Run a plan now", "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" }}], "responses": { "202": { "description": "Accepted" }, "409": { "description": "Rclone not ready or plan already running" }}}},
            "/api/notifications": {
                "get": { "summary": "Read masked global notification settings", "responses": { "200": { "description": "Settings and migration candidates" }}},
                "put": { "summary": "Save or confirm global notification settings", "responses": { "200": { "description": "Saved" }, "422": { "description": "Validation error" }}}
            },
            "/api/notifications/test": { "post": { "summary": "Send a channel test using global notification settings", "responses": { "200": { "description": "Delivered" }, "422": { "description": "Validation or delivery error" }}}},
            "/api/runs": { "get": { "summary": "List persistent run history", "responses": { "200": { "description": "Runs" }}}}
        }
    }))
}

async fn api_docs() -> impl IntoResponse {
    static_asset(
        include_str!("../../../web/api-docs.html"),
        "text/html; charset=utf-8",
    )
}

async fn list_plans(State(state): State<AppState>) -> ApiResult<Json<Vec<Plan>>> {
    let mut plans = state.store.list_plans().await?;
    for plan in &mut plans {
        redact_plan(plan);
    }
    Ok(Json(plans))
}

async fn create_plan(
    State(state): State<AppState>,
    Json(mut input): Json<PlanInput>,
) -> ApiResult<(StatusCode, Json<Plan>)> {
    input.notifications = NotificationConfig::default();
    input.validate().map_err(ApiError::validation)?;
    ensure_plan_aliases(&state, &input).await?;
    let now = Utc::now();
    let plan = input.into_plan(Uuid::new_v4(), now);
    state.store.save_plan(&plan).await?;
    let mut public = plan;
    redact_plan(&mut public);
    Ok((StatusCode::CREATED, Json(public)))
}

async fn update_plan(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut input): Json<PlanInput>,
) -> ApiResult<Json<Plan>> {
    input.validate().map_err(ApiError::validation)?;
    ensure_plan_aliases(&state, &input).await?;
    let existing = state
        .store
        .get_plan(id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    input.notifications.clone_from(&existing.notifications);
    merge_redacted_secrets(&mut input, &existing);
    let mut plan = input.into_plan(id, existing.created_at);
    plan.updated_at = Utc::now();
    state.store.save_plan(&plan).await?;
    redact_plan(&mut plan);
    Ok(Json(plan))
}

async fn delete_plan(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if state.runner.is_active(id).await {
        return Err(ApiError::conflict("plan is running"));
    }
    if state.store.delete_plan(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found())
    }
}

async fn reveal_archive_password(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    require_password_reveal_auth(state.public_auth.is_some())?;
    let plan = state
        .store
        .get_plan(id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    Ok(archive_password_response(&plan.archive.password))
}

fn require_password_reveal_auth(authentication_enabled: bool) -> ApiResult<()> {
    if !authentication_enabled {
        return Err(ApiError::forbidden(
            "enable web authentication before revealing archive passwords",
        ));
    }
    Ok(())
}

fn archive_password_response(password: &str) -> Response {
    let body = serde_json::to_vec(&json!({ "password": password }))
        .expect("archive password response is serializable");
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(body))
        .unwrap()
}

async fn run_plan(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let plan = state
        .store
        .get_plan(id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let run_id = state
        .runner
        .start(plan, "manual")
        .await
        .map_err(|e| ApiError::conflict(e.to_string()))?;
    Ok((StatusCode::ACCEPTED, Json(json!({ "run_id": run_id }))))
}

#[derive(Deserialize)]
struct RunQuery {
    plan_id: Option<Uuid>,
    limit: Option<u32>,
}
async fn list_runs(
    State(state): State<AppState>,
    Query(query): Query<RunQuery>,
) -> ApiResult<Json<Vec<RunRecord>>> {
    Ok(Json(
        state
            .store
            .list_runs(query.plan_id, query.limit.unwrap_or(50))
            .await?,
    ))
}

#[derive(Deserialize)]
struct NotificationUpdate {
    config: NotificationConfig,
}

#[derive(Deserialize)]
struct NotificationTestInput {
    target_id: String,
    #[serde(default)]
    config: Option<NotificationConfig>,
}

#[derive(Serialize)]
struct NotificationUpdateResponse {
    #[serde(flatten)]
    settings: GlobalNotificationSettings,
}

async fn get_notifications(
    State(state): State<AppState>,
) -> ApiResult<Json<GlobalNotificationSettings>> {
    let mut settings = state.store.notification_settings().await?;
    redact_notification_settings(&mut settings);
    Ok(Json(settings))
}

async fn update_notifications(
    State(state): State<AppState>,
    Json(input): Json<NotificationUpdate>,
) -> ApiResult<Json<NotificationUpdateResponse>> {
    let existing = state.store.notification_settings().await?;
    let mut config = input.config;
    config.merge_redacted_from(&existing.config);
    config.validate().map_err(ApiError::validation)?;
    config
        .validate_network_targets()
        .await
        .map_err(ApiError::validation)?;
    let mut settings = GlobalNotificationSettings {
        confirmed: true,
        config,
        updated_at: Utc::now(),
    };
    state.store.save_notification_settings(&settings).await?;
    redact_notification_settings(&mut settings);
    Ok(Json(NotificationUpdateResponse { settings }))
}

async fn test_notification(
    State(state): State<AppState>,
    Json(input): Json<NotificationTestInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let existing = state.store.notification_settings().await?;
    let mut config = input.config.unwrap_or_else(|| existing.config.clone());
    config.merge_redacted_from(&existing.config);
    config.validate().map_err(ApiError::validation)?;
    config
        .validate_network_targets()
        .await
        .map_err(ApiError::validation)?;
    state
        .runner
        .test_notification(&config, &input.target_id)
        .await
        .map_err(|_| ApiError::validation("notification test failed".into()))?;
    Ok(Json(json!({ "ok": true })))
}

fn redact_plan(plan: &mut Plan) {
    if !plan.archive.password.is_empty() {
        plan.archive.password = REDACTED.into();
    }
    redact_notification_config(&mut plan.notifications);
}

fn redact_notification_settings(settings: &mut GlobalNotificationSettings) {
    redact_notification_config(&mut settings.config);
}

fn redact_notification_config(config: &mut NotificationConfig) {
    for target in &mut config.targets {
        match &mut target.kind {
            NotificationTargetKind::ServerChan { config } => {
                if !config.send_key.is_empty() {
                    config.send_key = REDACTED.into();
                }
            }
            NotificationTargetKind::Email { config } => {
                if !config.smtp_options.is_empty() {
                    config.smtp_options = vec![REDACTED.into()];
                }
            }
            NotificationTargetKind::Ntfy { config } => {
                if !config.token.is_empty() {
                    config.token = REDACTED.into();
                }
            }
            NotificationTargetKind::Ping { config } => {
                for url in [
                    &mut config.completion_url,
                    &mut config.start_url,
                    &mut config.success_url,
                    &mut config.failure_url,
                ] {
                    if !url.is_empty() {
                        *url = REDACTED.into();
                    }
                }
                for options in [
                    &mut config.completion_options,
                    &mut config.start_options,
                    &mut config.success_options,
                    &mut config.failure_options,
                ] {
                    if !options.is_empty() {
                        *options = vec![REDACTED.into()];
                    }
                }
            }
        }
    }
    for url in [
        &mut config.ping.completion_url,
        &mut config.ping.start_url,
        &mut config.ping.success_url,
        &mut config.ping.failure_url,
    ] {
        if !url.is_empty() {
            *url = REDACTED.into();
        }
    }
    if !config.serverchan.send_key.is_empty() {
        config.serverchan.send_key = REDACTED.into();
    }
    if !config.mail.smtp_options.is_empty() {
        config.mail.smtp_options.clear();
        config.mail.smtp_options.push(REDACTED.into());
    }
    for values in [
        &mut config.ping.completion_options,
        &mut config.ping.start_options,
        &mut config.ping.success_options,
        &mut config.ping.failure_options,
    ] {
        if !values.is_empty() {
            values.clear();
            values.push(REDACTED.into());
        }
    }
}

fn merge_redacted_secrets(input: &mut PlanInput, existing: &Plan) {
    if input.archive.password == REDACTED {
        input
            .archive
            .password
            .clone_from(&existing.archive.password);
    }
    if input.notifications.serverchan.send_key == REDACTED {
        input
            .notifications
            .serverchan
            .send_key
            .clone_from(&existing.notifications.serverchan.send_key);
    }
    for (value, old) in input
        .notifications
        .mail
        .smtp_options
        .iter_mut()
        .zip(&existing.notifications.mail.smtp_options)
    {
        if value.ends_with(REDACTED) {
            value.clone_from(old);
        }
    }
}

async fn not_found() -> ApiError {
    ApiError::not_found()
}

type ApiResult<T> = Result<T, ApiError>;
pub struct ApiError {
    status: StatusCode,
    message: String,
}
impl ApiError {
    fn validation(message: String) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message,
        }
    }
    fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: "not found".into(),
        }
    }
    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }
    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }
}
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        tracing::error!(error = %error.into(), "internal API error");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "internal server error".into(),
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FRONTEND_ROUTES, NotificationUpdateResponse, archive_password_response,
        redact_notification_config, remote_needs_input, require_password_reveal_auth,
        same_origin_or_non_browser,
    };
    use axum::http::{HeaderMap, HeaderValue, header};
    use rclone_backup_core::{GlobalNotificationSettings, NotificationConfig, REDACTED};
    use serde_json::json;

    #[test]
    fn completed_rclone_response_is_not_interactive() {
        assert!(!remote_needs_input(&json!({
            "State": "",
            "Option": null,
            "Result": "",
            "Error": ""
        })));
    }

    #[test]
    fn frontend_history_routes_share_the_index_registration() {
        assert_eq!(
            FRONTEND_ROUTES,
            ["/", "/plans", "/accounts", "/notifications", "/history"]
        );
    }

    #[test]
    fn index_versions_frontend_assets_with_the_build_version() {
        let html = super::render_index_html("Backup");
        let version = env!("CARGO_PKG_VERSION");

        assert!(html.contains(&format!("/app.css?v={version}-2")));
        assert!(html.contains(&format!("/app.js?v={version}-2")));
        assert!(!html.contains("{{ASSET_VERSION}}"));
    }

    #[test]
    fn rclone_question_requires_continuation() {
        assert!(remote_needs_input(&json!({
            "State": "choose",
            "Option": { "Name": "answer" }
        })));
    }

    #[test]
    fn browser_mutations_must_be_same_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HOST,
            HeaderValue::from_static("backup.example:8080"),
        );
        assert!(same_origin_or_non_browser(&headers));
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://backup.example:8080"),
        );
        assert!(same_origin_or_non_browser(&headers));
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://attacker.example"),
        );
        assert!(!same_origin_or_non_browser(&headers));
    }

    #[test]
    fn archive_password_reveal_requires_web_authentication() {
        let error = require_password_reveal_auth(false).unwrap_err();
        assert_eq!(error.status, axum::http::StatusCode::FORBIDDEN);
        assert!(require_password_reveal_auth(true).is_ok());
    }

    #[tokio::test]
    async fn archive_password_response_is_never_cacheable() {
        let response = archive_password_response("recovery-secret");
        assert_eq!(
            response.headers()[axum::http::header::CACHE_CONTROL],
            "no-store"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({ "password": "recovery-secret" })
        );
    }

    #[test]
    fn notification_secrets_are_masked_and_redacted_values_merge() {
        let mut stored = NotificationConfig::default();
        stored.ping.enabled = true;
        stored.ping.success_url = "https://notify.example/private-token".into();
        stored.ping.success_options = vec!["--header".into(), "Authorization: Bearer token".into()];
        stored.mail.smtp_options = vec!["-S".into(), "smtp-auth-password=hunter2".into()];
        stored.serverchan.send_key = "SCTprivate".into();

        let mut public = stored.clone();
        redact_notification_config(&mut public);
        assert_eq!(public.ping.success_url, REDACTED);
        assert_eq!(public.ping.success_options, vec![REDACTED]);
        assert_eq!(public.mail.smtp_options, vec![REDACTED]);
        assert_eq!(public.serverchan.send_key, REDACTED);

        public.merge_redacted_from(&stored);
        assert_eq!(public, stored);

        stored.mail.smtp_options =
            vec!["-S".into(), "mta=smtps://alice:hunter2@smtp.example".into()];
        let mut public = stored.clone();
        redact_notification_config(&mut public);
        assert_eq!(public.mail.smtp_options, vec![REDACTED]);
    }

    #[test]
    fn notification_update_response_contains_the_saved_settings() {
        let response = NotificationUpdateResponse {
            settings: GlobalNotificationSettings::default(),
        };

        let value = serde_json::to_value(response).unwrap();

        assert!(value.get("config").is_some());
    }
}
