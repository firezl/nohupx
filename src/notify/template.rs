use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use minijinja::{context, Environment, Value};

use crate::config::{
    NotifyConfig, NotifyTargetConfig, NotifyTemplatesConfig, TargetTemplateFields,
    TemplatePartialConfig, TemplatePresetConfig, TemplateScenarioConfig,
};
use crate::runner::RunResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateScenario {
    Run,
    Test,
}

#[derive(Debug, Clone)]
pub struct TemplateContext {
    pub success: bool,
    pub exit_code: i32,
    pub name: String,
    pub command: String,
    pub host: String,
    pub duration_seconds: u64,
    pub started_at: String,
    pub finished_at: String,
    pub log_path: PathBuf,
    pub tail: String,
    pub tail_lines: usize,
    pub is_test: bool,
    pub now: String,
    pub config_path: String,
    pub target_name: String,
    pub target_type: String,
    pub target_label: String,
}

impl TemplateContext {
    pub fn from_run_result(result: &RunResult) -> Self {
        Self {
            success: result.success,
            exit_code: result.exit_code,
            name: result.name.clone().unwrap_or_else(|| "-".to_string()),
            command: result.command.clone(),
            host: result.host.clone(),
            duration_seconds: result.duration_seconds,
            started_at: result.started_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            finished_at: result.finished_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            log_path: result.log_path.clone(),
            tail: result.tail.clone(),
            tail_lines: result.tail_lines,
            is_test: false,
            now: String::new(),
            config_path: String::new(),
            target_name: String::new(),
            target_type: String::new(),
            target_label: String::new(),
        }
    }

    pub fn for_test(
        host: &str,
        now: &str,
        config_path: &Path,
        target_name: &str,
        target_type: &str,
        target_label: &str,
    ) -> Self {
        Self {
            success: true,
            exit_code: 0,
            name: "-".to_string(),
            command: "nohupx test".to_string(),
            host: host.to_string(),
            duration_seconds: 0,
            started_at: String::new(),
            finished_at: String::new(),
            log_path: PathBuf::new(),
            tail: String::new(),
            tail_lines: 0,
            is_test: true,
            now: now.to_string(),
            config_path: config_path.display().to_string(),
            target_name: target_name.to_string(),
            target_type: target_type.to_string(),
            target_label: target_label.to_string(),
        }
    }

    fn icon(&self) -> &'static str {
        if self.success {
            "✅"
        } else {
            "❌"
        }
    }

    fn status(&self) -> &'static str {
        if self.success {
            "finished"
        } else {
            "failed"
        }
    }

    fn action(&self) -> &'static str {
        self.status()
    }

    fn display_name(&self) -> &str {
        if self.name == "-" {
            ""
        } else {
            &self.name
        }
    }

    fn to_jinja_context(&self, include_tail: bool) -> Value {
        let tail = if include_tail {
            self.tail.clone()
        } else {
            String::new()
        };
        context! {
            success => self.success,
            exit_code => self.exit_code,
            name => self.name.clone(),
            command => self.command.clone(),
            host => self.host.clone(),
            duration_seconds => self.duration_seconds,
            started_at => self.started_at.clone(),
            finished_at => self.finished_at.clone(),
            log_path => self.log_path.display().to_string(),
            tail => tail,
            tail_lines => self.tail_lines,
            icon => self.icon(),
            status => self.status(),
            action => self.action(),
            display_name => self.display_name().to_string(),
            is_test => self.is_test,
            now => self.now.clone(),
            config_path => self.config_path.clone(),
            target_name => self.target_name.clone(),
            target_type => self.target_type.clone(),
            target_label => self.target_label.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemplateSettings {
    pub title: String,
    pub body: String,
    pub include_tail: bool,
    pub attach_log: bool,
}

#[derive(Debug, Clone)]
pub struct RenderedMessage {
    pub title: String,
    pub body: String,
    pub attach_log: bool,
    pub log_path: PathBuf,
    pub success: bool,
    pub exit_code: i32,
    pub command: String,
    pub host: String,
    pub duration_seconds: u64,
}

pub fn resolve_settings(
    notify: &NotifyConfig,
    target: &NotifyTargetConfig,
    scenario: TemplateScenario,
) -> TemplateSettings {
    let templates = &notify.templates;
    let mut settings = builtin_settings(scenario);

    if let Some(global) = global_scenario_config(templates, scenario) {
        merge_scenario(&mut settings, global);
    }

    if let Some(type_cfg) = templates.types.get(target.type_name()) {
        merge_partial(&mut settings, type_cfg);
    }

    if let Some(preset_name) = target.template_fields().template_preset.as_deref() {
        if let Some(preset) = templates.presets.get(preset_name) {
            if let Some(preset_scenario) = preset_scenario_config(preset, scenario) {
                merge_scenario(&mut settings, preset_scenario);
            }
        }
    }

    merge_target_override(&mut settings, target.template_fields());

    settings
}

pub fn render(settings: &TemplateSettings, ctx: &TemplateContext) -> Result<RenderedMessage> {
    let env = Environment::new();
    let jinja_ctx = ctx.to_jinja_context(settings.include_tail);

    let title = env
        .template_from_str(&settings.title)
        .context("invalid title template")?
        .render(jinja_ctx.clone())
        .context("failed to render title template")?;

    let body = env
        .template_from_str(&settings.body)
        .context("invalid body template")?
        .render(jinja_ctx)
        .context("failed to render body template")?;

    Ok(RenderedMessage {
        title,
        body,
        attach_log: settings.attach_log,
        log_path: ctx.log_path.clone(),
        success: ctx.success,
        exit_code: ctx.exit_code,
        command: ctx.command.clone(),
        host: ctx.host.clone(),
        duration_seconds: ctx.duration_seconds,
    })
}

pub fn render_for_target(
    notify: &NotifyConfig,
    target: &NotifyTargetConfig,
    ctx: &TemplateContext,
    scenario: TemplateScenario,
) -> Result<RenderedMessage> {
    let settings = resolve_settings(notify, target, scenario);
    render(&settings, ctx)
}

pub fn max_attachment_bytes(notify: &NotifyConfig) -> u64 {
    notify.templates.max_attachment_bytes
}

fn global_scenario_config(
    templates: &NotifyTemplatesConfig,
    scenario: TemplateScenario,
) -> Option<&TemplateScenarioConfig> {
    match scenario {
        TemplateScenario::Run => templates.run.as_ref(),
        TemplateScenario::Test => templates.test.as_ref(),
    }
}

fn preset_scenario_config(
    preset: &TemplatePresetConfig,
    scenario: TemplateScenario,
) -> Option<&TemplateScenarioConfig> {
    match scenario {
        TemplateScenario::Run => preset.run.as_ref(),
        TemplateScenario::Test => preset.test.as_ref(),
    }
}

fn builtin_settings(scenario: TemplateScenario) -> TemplateSettings {
    match scenario {
        TemplateScenario::Run => TemplateSettings {
            title: builtin_run_title().to_string(),
            body: builtin_run_body().to_string(),
            include_tail: true,
            attach_log: false,
        },
        TemplateScenario::Test => TemplateSettings {
            title: builtin_test_title().to_string(),
            body: builtin_test_body().to_string(),
            include_tail: false,
            attach_log: false,
        },
    }
}

fn merge_scenario(settings: &mut TemplateSettings, cfg: &TemplateScenarioConfig) {
    if let Some(title) = &cfg.title {
        settings.title = title.clone();
    }
    if let Some(body) = &cfg.body {
        settings.body = body.clone();
    }
    if let Some(include_tail) = cfg.include_tail {
        settings.include_tail = include_tail;
    }
    if let Some(attach_log) = cfg.attach_log {
        settings.attach_log = attach_log;
    }
}

fn merge_partial(settings: &mut TemplateSettings, cfg: &TemplatePartialConfig) {
    if let Some(title) = &cfg.title {
        settings.title = title.clone();
    }
    if let Some(body) = &cfg.body {
        settings.body = body.clone();
    }
    if let Some(include_tail) = cfg.include_tail {
        settings.include_tail = include_tail;
    }
    if let Some(attach_log) = cfg.attach_log {
        settings.attach_log = attach_log;
    }
}

fn merge_target_override(settings: &mut TemplateSettings, fields: &TargetTemplateFields) {
    if let Some(title) = &fields.title_template {
        settings.title = title.clone();
    }
    if let Some(body) = &fields.body_template {
        settings.body = body.clone();
    }
    if let Some(include_tail) = fields.include_tail {
        settings.include_tail = include_tail;
    }
    if let Some(attach_log) = fields.attach_log {
        settings.attach_log = attach_log;
    }
}

fn builtin_run_title() -> &'static str {
    "{% if display_name %}{{ icon }} {{ display_name }} {{ status }} on {{ host }}{% else %}{{ icon }} Command {{ status }} on {{ host }}{% endif %}"
}

fn builtin_run_body() -> &'static str {
    "Name:\n{{ name }}\n\nCommand:\n{{ command }}\n\nExit code:\n{{ exit_code }}\n\nDuration:\n{{ duration_seconds }}s\n\nStarted at:\n{{ started_at }}\n\nFinished at:\n{{ finished_at }}\n\nHost:\n{{ host }}\n\nLog:\n{{ log_path }}\n\nLast {{ tail_lines }} lines:\n{{ tail }}"
}

fn builtin_test_title() -> &'static str {
    "🔔 nohupx test notification"
}

fn builtin_test_body() -> &'static str {
    "This is a test notification from nohupx.\n\nHost:\n{{ host }}\n\nTime:\n{{ now }}\n\nConfig:\n{{ config_path }}\n\nTarget:\n{{ target_label }}"
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;
    use crate::config::{NotifyConfig, NotifyTargetConfig};

    fn legacy_run_title(result: &RunResult) -> String {
        let action = if result.success { "finished" } else { "failed" };
        let icon = if result.success { "✅" } else { "❌" };
        if let Some(name) = &result.name {
            format!("{icon} {name} {action} on {}", result.host)
        } else {
            format!("{icon} Command {action} on {}", result.host)
        }
    }

    fn legacy_run_body(result: &RunResult) -> String {
        let display_name = result.name.as_deref().unwrap_or("-");
        format!(
            "Name:\n{display_name}\n\nCommand:\n{}\n\nExit code:\n{}\n\nDuration:\n{}s\n\nStarted at:\n{}\n\nFinished at:\n{}\n\nHost:\n{}\n\nLog:\n{}\n\nLast {} lines:\n{}",
            result.command,
            result.exit_code,
            result.duration_seconds,
            result.started_at.format("%Y-%m-%d %H:%M:%S"),
            result.finished_at.format("%Y-%m-%d %H:%M:%S"),
            result.host,
            result.log_path.display(),
            result.tail_lines,
            result.tail
        )
    }

    fn sample_run_result(success: bool) -> RunResult {
        RunResult {
            name: Some("exp01".to_string()),
            command: "python train.py".to_string(),
            exit_code: if success { 0 } else { 1 },
            success,
            started_at: Local::now(),
            finished_at: Local::now(),
            duration_seconds: 42,
            host: "lab-server".to_string(),
            log_path: PathBuf::from("/tmp/run.log"),
            tail_lines: 3,
            tail: "line1\nline2\nline3".to_string(),
        }
    }

    fn email_target() -> NotifyTargetConfig {
        NotifyTargetConfig::Email {
            template: TargetTemplateFields::default(),
            name: Some("my-email".to_string()),
            enabled: Some(true),
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: Some(587),
            username: "u".to_string(),
            password_secret: None,
            password_env: None,
            password: Some("p".to_string()),
            from: "a@example.com".to_string(),
            to: vec!["b@example.com".to_string()],
        }
    }

    #[test]
    fn builtin_run_template_matches_legacy_success() {
        let result = sample_run_result(true);
        let ctx = TemplateContext::from_run_result(&result);
        let rendered = render(&builtin_settings(TemplateScenario::Run), &ctx).unwrap();
        assert_eq!(rendered.title, legacy_run_title(&result));
        assert_eq!(rendered.body, legacy_run_body(&result));
    }

    #[test]
    fn builtin_run_template_matches_legacy_failure() {
        let result = sample_run_result(false);
        let ctx = TemplateContext::from_run_result(&result);
        let rendered = render(&builtin_settings(TemplateScenario::Run), &ctx).unwrap();
        assert_eq!(rendered.title, legacy_run_title(&result));
        assert_eq!(rendered.body, legacy_run_body(&result));
    }

    #[test]
    fn include_tail_false_clears_tail_in_context() {
        let result = sample_run_result(true);
        let ctx = TemplateContext::from_run_result(&result);
        let mut settings = builtin_settings(TemplateScenario::Run);
        settings.include_tail = false;
        let rendered = render(&settings, &ctx).unwrap();
        assert!(!rendered.body.contains("line1"));
        assert!(rendered.body.contains("Last 3 lines:"));
    }

    #[test]
    fn target_title_override_wins() {
        let mut notify = NotifyConfig::default();
        notify.templates.types.insert(
            "email".to_string(),
            TemplatePartialConfig {
                title: Some("type-title".to_string()),
                ..Default::default()
            },
        );
        let mut target = email_target();
        if let NotifyTargetConfig::Email { template, .. } = &mut target {
            template.title_template = Some("target-title {{ name }}".to_string());
        }
        let settings = resolve_settings(&notify, &target, TemplateScenario::Run);
        assert_eq!(settings.title, "target-title {{ name }}");
    }

    #[test]
    fn preset_merge_applies_before_target_override() {
        let mut notify = NotifyConfig::default();
        notify.templates.presets.insert(
            "minimal".to_string(),
            TemplatePresetConfig {
                run: Some(TemplateScenarioConfig {
                    title: Some("preset {{ status }}".to_string()),
                    body: Some("cmd={{ command }}".to_string()),
                    include_tail: Some(false),
                    attach_log: Some(true),
                }),
                test: None,
            },
        );
        let mut target = email_target();
        if let NotifyTargetConfig::Email { template, .. } = &mut target {
            template.template_preset = Some("minimal".to_string());
            template.attach_log = Some(false);
        }
        let settings = resolve_settings(&notify, &target, TemplateScenario::Run);
        assert_eq!(settings.title, "preset {{ status }}");
        assert_eq!(settings.body, "cmd={{ command }}");
        assert!(!settings.include_tail);
        assert!(!settings.attach_log);
    }

    #[test]
    fn conditional_template_branch() {
        let result = sample_run_result(false);
        let ctx = TemplateContext::from_run_result(&result);
        let settings = TemplateSettings {
            title: "{% if success %}OK{% else %}FAIL{% endif %}".to_string(),
            body: "{{ command }}".to_string(),
            include_tail: true,
            attach_log: false,
        };
        let rendered = render(&settings, &ctx).unwrap();
        assert_eq!(rendered.title, "FAIL");
        assert_eq!(rendered.body, "python train.py");
    }
}
