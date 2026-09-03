use std::io::Write;

use clap::{Arg, ArgAction, ArgGroup, Command, ValueHint};
use rust_i18n::t;

pub(super) fn command() -> Command {
    let command = Command::new("herdr")
        .about(t!("cli.herdr_about").to_string())
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(help_flag())
        .arg(option("session", "NAME").help(t!("cli.session_help").to_string()))
        .arg(option("remote", "TARGET").help(t!("cli.remote_help").to_string()))
        .arg(
            option("remote-keybindings", "MODE")
                .value_parser(["local", "server"])
                .help(t!("cli.remote_keybindings_help").to_string()),
        )
        .arg(flag("handoff").help(t!("cli.handoff_help").to_string()))
        .arg(flag("default-config").help(t!("cli.default_config_help").to_string()))
        .arg(flag("skill").help(t!("cli.skill_help").to_string()))
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue)
                .help(t!("cli.version_help").to_string()),
        )
        .subcommand(completion_command())
        .subcommand(update_command())
        .subcommand(status_command())
        .subcommand(config_command())
        .subcommand(channel_command())
        .subcommand(server_command())
        .subcommand(api_command())
        .subcommand(workspace_command())
        .subcommand(worktree_command())
        .subcommand(tab_command())
        .subcommand(notification_command())
        .subcommand(agent_command())
        .subcommand(pane_command())
        .subcommand(terminal_command())
        .subcommand(session_command())
        .subcommand(integration_command())
        .subcommand(plugin_command());
    configure_help(command, 0)
}

fn configure_help(command: Command, depth: usize) -> Command {
    let command = if depth == 0 {
        command
    } else {
        command.disable_help_flag(false)
    };
    let command = if depth == 1 && command.has_subcommands() {
        command.after_help(super::AGENT_HELP_FOOTER)
    } else {
        command
    };
    command
        .disable_help_subcommand(true)
        .mut_subcommands(|subcommand| configure_help(subcommand, depth + 1))
}

pub(super) fn print_requested_help(args: &[String]) -> std::io::Result<bool> {
    let mut stdout = std::io::stdout().lock();
    write_requested_help(args, &mut stdout, crate::platform::begin_cli_output)
}

fn write_requested_help(
    args: &[String],
    output: &mut impl Write,
    before_write: impl FnOnce(),
) -> std::io::Result<bool> {
    let Some(help_index) = args
        .iter()
        .position(|arg| matches!(arg.as_str(), "--help" | "-h"))
    else {
        return Ok(false);
    };
    if help_index < 2 {
        return Ok(false);
    }
    if args[1..help_index].iter().any(|arg| arg == "--") {
        return Ok(false);
    }

    let mut root = command();
    root.build();
    let mut selected = &mut root;
    let mut path = vec!["herdr".to_string()];
    for segment in &args[1..help_index] {
        if selected.find_subcommand(segment).is_none() {
            break;
        }
        path.push(segment.clone());
        selected = selected
            .find_subcommand_mut(segment)
            .expect("subcommand checked immediately before mutable lookup");
    }
    if path.len() == 1 || help_index != path.len() {
        return Ok(false);
    }

    selected.set_bin_name(path.join(" "));
    before_write();
    selected.write_long_help(&mut *output)?;
    writeln!(output)?;
    Ok(true)
}

fn completion_command() -> Command {
    Command::new("completion")
        .visible_alias("completions")
        .about(t!("cli.completion_about").to_string())
        .arg(
            Arg::new("shell")
                .value_name("SHELL")
                .required(true)
                .value_parser(super::completion::SUPPORTED_SHELLS)
                .help(t!("cli.completion_shell_help").to_string()),
        )
}

fn update_command() -> Command {
    Command::new("update")
        .about(t!("cli.update_about").to_string())
        .arg(flag("handoff").help(t!("cli.update_handoff_help").to_string()))
}

fn status_command() -> Command {
    Command::new("status")
        .about(t!("cli.status_about").to_string())
        .arg(json_flag())
        .subcommand(
            Command::new("server")
                .about(t!("cli.status_server_about").to_string())
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("client")
                .about(t!("cli.status_client_about").to_string())
                .arg(json_flag()),
        )
}

fn config_command() -> Command {
    Command::new("config")
        .about(t!("cli.config_about").to_string())
        .subcommand(Command::new("check").about(t!("cli.config_check_about").to_string()))
        .subcommand(Command::new("reset-keys").about(t!("cli.config_reset_keys_about").to_string()))
}

fn channel_command() -> Command {
    Command::new("channel")
        .about(t!("cli.channel_about").to_string())
        .subcommand(Command::new("show").about(t!("cli.channel_show_about").to_string()))
        .subcommand(
            Command::new("set")
                .about(t!("cli.channel_set_about").to_string())
                .arg(
                    Arg::new("channel")
                        .value_name("CHANNEL")
                        .required(true)
                        .value_parser(["stable", "preview"]),
                ),
        )
}

fn server_command() -> Command {
    Command::new("server")
        .about(t!("cli.server_about").to_string())
        .subcommand(Command::new("stop").about(t!("cli.server_stop_about").to_string()))
        .subcommand(
            Command::new("reload-config").about(t!("cli.server_reload_config_about").to_string()),
        )
        .subcommand(
            Command::new("agent-manifests")
                .about(t!("cli.server_agent_manifests_about").to_string())
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("update-agent-manifests")
                .about(t!("cli.server_update_agent_manifests_about").to_string())
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("reload-agent-manifests")
                .about(t!("cli.server_reload_agent_manifests_about").to_string()),
        )
}

fn api_command() -> Command {
    Command::new("api")
        .about(t!("cli.api_about").to_string())
        .subcommand(Command::new("snapshot").about(t!("cli.api_snapshot_about").to_string()))
        .subcommand(
            Command::new("schema")
                .about(t!("cli.api_schema_about").to_string())
                .arg(json_flag())
                .arg(path_option("output", "PATH")),
        )
}

fn workspace_command() -> Command {
    Command::new("workspace")
        .about(t!("cli.workspace_about").to_string())
        .subcommand(Command::new("list").about(t!("cli.workspace_list_about").to_string()))
        .subcommand(
            Command::new("create")
                .about(t!("cli.workspace_create_about").to_string())
                .arg(path_option("cwd", "PATH"))
                .arg(option("label", "TEXT"))
                .arg(env_option())
                .arg(flag("focus"))
                .arg(flag("no-focus")),
        )
        .subcommand(id_command(
            "get",
            "workspace_id",
            t!("cli.workspace_get_about").to_string(),
        ))
        .subcommand(id_command(
            "focus",
            "workspace_id",
            t!("cli.workspace_focus_about").to_string(),
        ))
        .subcommand(
            Command::new("rename")
                .about(t!("cli.workspace_rename_about").to_string())
                .arg(required("workspace_id", "WORKSPACE_ID"))
                .arg(required("label", "LABEL").num_args(1..)),
        )
        .subcommand(
            Command::new("report-metadata")
                .about(t!("cli.workspace_report_metadata_about").to_string())
                .arg(required("workspace_id", "WORKSPACE_ID"))
                .arg(option("source", "ID").required(true))
                .arg(repeatable_option("token", "NAME=VALUE"))
                .arg(repeatable_option("clear-token", "NAME"))
                .arg(option("seq", "N"))
                .arg(option("ttl-ms", "N")),
        )
        .subcommand(id_command(
            "close",
            "workspace_id",
            t!("cli.workspace_close_about").to_string(),
        ))
}

fn worktree_command() -> Command {
    Command::new("worktree")
        .about(t!("cli.worktree_about").to_string())
        .subcommand(
            Command::new("list")
                .about(t!("cli.worktree_list_about").to_string())
                .arg(option("workspace", "ID"))
                .arg(path_option("cwd", "PATH"))
                .arg(flag("trust-repository")),
        )
        .subcommand(
            Command::new("create")
                .about(t!("cli.worktree_create_about").to_string())
                .arg(option("workspace", "ID"))
                .arg(path_option("cwd", "PATH"))
                .arg(option("branch", "NAME"))
                .arg(option("base", "REF"))
                .arg(path_option("path", "PATH"))
                .arg(option("label", "TEXT"))
                .arg(flag("focus"))
                .arg(flag("no-focus"))
                .arg(flag("trust-repository")),
        )
        .subcommand(
            Command::new("open")
                .about(t!("cli.worktree_open_about").to_string())
                .arg(option("workspace", "ID"))
                .arg(path_option("cwd", "PATH"))
                .arg(path_option("path", "PATH"))
                .arg(option("branch", "NAME"))
                .arg(option("label", "TEXT"))
                .arg(flag("focus"))
                .arg(flag("no-focus"))
                .arg(flag("trust-repository")),
        )
        .subcommand(
            Command::new("remove")
                .about(t!("cli.worktree_remove_about").to_string())
                .arg(option("workspace", "ID"))
                .arg(flag("force"))
                .arg(flag("trust-repository")),
        )
}

fn tab_command() -> Command {
    Command::new("tab")
        .about(t!("cli.tab_about").to_string())
        .subcommand(
            Command::new("list")
                .about(t!("cli.tab_list_about").to_string())
                .arg(option("workspace", "WORKSPACE_ID")),
        )
        .subcommand(
            Command::new("create")
                .about(t!("cli.tab_create_about").to_string())
                .arg(option("workspace", "WORKSPACE_ID"))
                .arg(path_option("cwd", "PATH"))
                .arg(option("label", "TEXT"))
                .arg(env_option())
                .arg(flag("focus"))
                .arg(flag("no-focus")),
        )
        .subcommand(id_command(
            "get",
            "tab_id",
            t!("cli.tab_get_about").to_string(),
        ))
        .subcommand(id_command(
            "focus",
            "tab_id",
            t!("cli.tab_focus_about").to_string(),
        ))
        .subcommand(
            Command::new("rename")
                .about(t!("cli.tab_rename_about").to_string())
                .arg(required("tab_id", "TAB_ID"))
                .arg(required("label", "LABEL").num_args(1..)),
        )
        .subcommand(id_command(
            "close",
            "tab_id",
            t!("cli.tab_close_about").to_string(),
        ))
}

fn notification_command() -> Command {
    Command::new("notification")
        .about(t!("cli.notification_about").to_string())
        .subcommand(
            Command::new("show")
                .about(t!("cli.notification_show_about").to_string())
                .arg(required("title", "TITLE"))
                .arg(option("body", "TEXT"))
                .arg(option("position", "POSITION").value_parser([
                    "top-left",
                    "top-right",
                    "bottom-left",
                    "bottom-right",
                ]))
                .arg(option("sound", "SOUND").value_parser(["none", "done", "request"])),
        )
}

fn agent_command() -> Command {
    Command::new("agent")
        .about(t!("cli.agent_about").to_string())
        .subcommand(Command::new("list").about(t!("cli.agent_list_about").to_string()))
        .subcommand(id_command("get", "target", t!("cli.agent_get_about").to_string()))
        .subcommand(
            Command::new("read")
                .about(t!("cli.agent_read_about").to_string())
                .override_usage("herdr agent read <TARGET> [OPTIONS]")
                .arg(required("target", "TARGET"))
                .arg(read_source_option(true))
                .arg(option("lines", "N"))
                .arg(text_ansi_format_option())
                .arg(flag("ansi")),
        )
        .subcommand(
            Command::new("send-keys")
                .about(t!("cli.agent_send_keys_about").to_string())
                .arg(required("target", "TARGET"))
                .arg(required("key", "KEY").num_args(1..))
                .after_help("Use esc as the canonical Escape key name; escape is also accepted."),
        )
        .subcommand(
            Command::new("prompt")
                .about(t!("cli.agent_prompt_about").to_string())
                .override_usage("herdr agent prompt <TARGET> <TEXT> [OPTIONS]")
                .arg(required("target", "TARGET"))
                .arg(required("text", "TEXT"))
                .arg(
                    flag("wait")
                        .help(t!("cli.agent_prompt_wait_help").to_string()),
                )
                .arg(
                    option("until", "STATUS")
                        .action(ArgAction::Append)
                        .requires("wait")
                        .value_parser(["idle", "working", "blocked", "done", "unknown"])
                        .help(t!("cli.agent_prompt_until_help").to_string()),
                )
                .arg(
                    option("timeout", "MS")
                        .requires("wait")
                        .help(t!("cli.agent_wait_timeout_help").to_string()),
                )
                .after_help(
                    "If the agent is already blocked, submission is rejected with agent_blocked before any input is sent. When an accepted submission starts from another non-working state, --wait requires an observed working or blocked state within 5000ms; otherwise it returns agent_prompt_stalled. A caller timeout that expires first returns timeout. It then matches idle, done, or blocked by default, or any exact --until state. It does not track turns: if the agent is already working, that active turn's completion may match.",
                ),
        )
        .subcommand(
            Command::new("rename")
                .about(t!("cli.agent_rename_about").to_string())
                .override_usage("herdr agent rename <TARGET> <NAME>|--clear")
                .arg(required("target", "TARGET"))
                .arg(Arg::new("name").value_name("NAME"))
                .arg(flag("clear"))
                .group(
                    ArgGroup::new("rename")
                        .args(["name", "clear"])
                        .required(true),
                ),
        )
        .subcommand(id_command("focus", "target", t!("cli.agent_focus_about").to_string()))
        .subcommand(
            Command::new("wait")
                .about(t!("cli.agent_wait_states_about").to_string())
                .override_usage("herdr agent wait <TARGET> [OPTIONS]")
                .arg(required("target", "TARGET"))
                .arg(
                    option("until", "STATUS")
                        .action(ArgAction::Append)
                        .value_parser(["idle", "working", "blocked", "done", "unknown"])
                        .help(t!("cli.agent_wait_until_help").to_string()),
                )
                .arg(option("timeout", "MS").help(t!("cli.agent_wait_timeout_help").to_string()))
                .after_help(
                    "Without --until, matches idle, done, or blocked. Use --until unknown explicitly when needed. Without --timeout, waits indefinitely.",
                ),
        )
        .subcommand(
            Command::new("attach")
                .about(t!("cli.agent_attach_about").to_string())
                .override_usage("herdr agent attach <TARGET> [OPTIONS]")
                .arg(required("target", "TARGET"))
                .arg(flag("takeover")),
        )
        .subcommand(
            Command::new("start")
                .about(t!("cli.agent_start_interactive_about").to_string())
                .override_usage(
                    "herdr agent start <NAME> --kind <KIND> --pane <ID> [OPTIONS] [-- [AGENT_ARG]...]",
                )
                .arg(required("name", "NAME"))
                .arg(
                    option("kind", "KIND")
                        .required(true)
                        .value_parser(agent_kind_values())
                        .help(t!("cli.agent_start_kind_help").to_string()),
                )
                .arg(
                    option("pane", "ID")
                        .required(true)
                        .help(t!("cli.agent_start_pane_help").to_string()),
                )
                .arg(
                    option("timeout", "MS")
                        .help(t!("cli.agent_start_ready_help").to_string()),
                )
                .arg(
                    Arg::new("agent_args")
                        .value_name("AGENT_ARG")
                        .num_args(0..)
                        .last(true),
                )
                .after_help(
                    "The pane must be at its interactive shell prompt. Success means the expected agent was detected in the same terminal and is ready for input.\n\nnext: herdr agent prompt <TARGET> <TEXT> --wait",
                ),
        )
        .subcommand(
            Command::new("explain")
                .about(t!("cli.agent_explain_about").to_string())
                .arg(Arg::new("target").value_name("TARGET"))
                .arg(path_option("file", "PATH"))
                .arg(option("agent", "LABEL"))
                .arg(json_flag())
                .arg(text_json_format_option())
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(ArgAction::SetTrue),
                ),
        )
}

pub(super) fn agent_kind_values() -> Vec<&'static str> {
    crate::detect::Agent::ALL
        .into_iter()
        .map(crate::detect::agent_label)
        .collect()
}

fn pane_command() -> Command {
    Command::new("pane")
        .about(t!("cli.pane_about").to_string())
        .subcommand(
            Command::new("list")
                .about(t!("cli.pane_list_about").to_string())
                .arg(option("workspace", "WORKSPACE_ID")),
        )
        .subcommand(
            Command::new("current")
                .about(t!("cli.pane_current_about").to_string())
                .args(current_pane_args()),
        )
        .subcommand(id_command("get", "pane_id", t!("cli.pane_get_about").to_string()))
        .subcommand(
            Command::new("layout")
                .about(t!("cli.pane_layout_about").to_string())
                .args(current_pane_args()),
        )
        .subcommand(
            Command::new("process-info")
                .about(t!("cli.pane_process_info_about").to_string())
                .args(current_pane_args()),
        )
        .subcommand(
            Command::new("neighbor")
                .about(t!("cli.pane_neighbor_about").to_string())
                .arg(required_direction_option())
                .args(current_pane_args()),
        )
        .subcommand(
            Command::new("edges")
                .about(t!("cli.pane_edges_about").to_string())
                .args(current_pane_args()),
        )
        .subcommand(
            Command::new("focus")
                .about(t!("cli.pane_focus_about").to_string())
                .arg(required_direction_option())
                .args(current_pane_args()),
        )
        .subcommand(
            Command::new("resize")
                .about(t!("cli.pane_resize_about").to_string())
                .arg(required_direction_option())
                .arg(option("amount", "FLOAT"))
                .args(current_pane_args()),
        )
        .subcommand(
            Command::new("zoom")
                .about(t!("cli.pane_zoom_about").to_string())
                .arg(Arg::new("pane_id").value_name("PANE_ID"))
                .args(current_pane_args())
                .arg(flag("toggle"))
                .arg(flag("on"))
                .arg(flag("off")),
        )
        .subcommand(
            Command::new("read")
                .about(t!("cli.pane_read_about").to_string())
                .arg(required("pane_id", "PANE_ID"))
                .arg(read_source_option(true))
                .arg(option("lines", "N"))
                .arg(text_ansi_format_option())
                .arg(flag("ansi"))
                .arg(flag("raw")),
        )
        .subcommand(
            Command::new("rename")
                .about(t!("cli.pane_rename_about").to_string())
                .arg(required("pane_id", "PANE_ID"))
                .arg(Arg::new("label").value_name("LABEL").num_args(1..))
                .arg(flag("clear")),
        )
        .subcommand(
            Command::new("input")
                .about(t!("cli.pane_input_about").to_string())
                .arg(Arg::new("pane_id").value_name("PANE_ID"))
                .args(current_pane_args())
                .arg(
                    option("right-click", "TARGET")
                        .value_parser(["herdr", "pane"])
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("split")
                .about(t!("cli.pane_split_about").to_string())
                .arg(Arg::new("pane_id").value_name("PANE_ID"))
                .args(current_pane_args())
                .arg(split_direction_option())
                .arg(option("ratio", "FLOAT"))
                .arg(path_option("cwd", "PATH"))
                .arg(env_option())
                .arg(option("right-click", "TARGET").value_parser(["herdr", "pane"]))
                .arg(flag("focus"))
                .arg(flag("no-focus")),
        )
        .subcommand(
            Command::new("swap")
                .about(t!("cli.pane_swap_about").to_string())
                .arg(direction_option())
                .args(current_pane_args())
                .arg(option("source-pane", "ID"))
                .arg(option("target-pane", "ID")),
        )
        .subcommand(
            Command::new("move")
                .about(t!("cli.pane_move_about").to_string())
                .arg(required("pane_id", "PANE_ID"))
                .arg(option("tab", "TAB_ID"))
                .arg(option("split", "DIRECTION").value_parser(["right", "down"]))
                .arg(option("target-pane", "ID"))
                .arg(option("ratio", "FLOAT"))
                .arg(flag("new-tab"))
                .arg(option("workspace", "ID"))
                .arg(flag("new-workspace"))
                .arg(option("label", "TEXT"))
                .arg(option("tab-label", "TEXT"))
                .arg(flag("focus"))
                .arg(flag("no-focus")),
        )
        .subcommand(id_command("close", "pane_id", t!("cli.pane_close_about").to_string()))
        .subcommand(
            Command::new("send-text")
                .about(t!("cli.pane_send_text_about").to_string())
                .arg(required("pane_id", "PANE_ID"))
                .arg(required("text", "TEXT"))
                .after_help(
                    "next: herdr pane run <PANE_ID> <COMMAND> sends text and Enter in one call",
                ),
        )
        .subcommand(
            Command::new("send-keys")
                .about(t!("cli.pane_send_keys_about").to_string())
                .arg(required("pane_id", "PANE_ID"))
                .arg(required("key", "KEY").num_args(1..))
                .after_help("Use esc as the canonical Escape key name; escape is also accepted."),
        )
        .subcommand(
            Command::new("wait-output")
                .about(t!("cli.wait_output_about").to_string())
                .arg(required("pane_id", "PANE_ID"))
                .arg(
                    option("match", "TEXT")
                        .conflicts_with("regex")
                        .required_unless_present("regex")
                        .help(t!("cli.pane_match_literal_help").to_string()),
                )
                .arg(
                    option("regex", "PATTERN")
                        .conflicts_with("match")
                        .required_unless_present("match")
                        .help(t!("cli.pane_match_regex_help").to_string()),
                )
                .arg(read_source_option(false))
                .arg(option("lines", "N").help(t!("cli.pane_match_lines_help").to_string()))
                .arg(option("timeout", "MS").help(t!("cli.agent_wait_timeout_help").to_string()))
                .arg(flag("raw").help(t!("cli.pane_match_raw_help").to_string()))
                .group(
                    ArgGroup::new("matcher")
                        .args(["match", "regex"])
                        .required(true),
                )
                .after_help(
                    "The selected snapshot is searched immediately, including existing output, then polled. Without --timeout, this waits indefinitely.",
                ),
        )
        .subcommand(
            Command::new("run")
                .about(t!("cli.pane_run_about").to_string())
                .arg(required("pane_id", "PANE_ID"))
                .arg(required("command", "COMMAND").num_args(1..)),
        )
        .subcommand(report_agent_command())
        .subcommand(report_agent_session_command())
        .subcommand(release_agent_command())
        .subcommand(report_metadata_command())
}

fn report_agent_command() -> Command {
    Command::new("report-agent")
        .about(t!("cli.pane_report_agent_about").to_string())
        .arg(required("pane_id", "PANE_ID"))
        .arg(option("source", "ID").required(true))
        .arg(option("agent", "LABEL").required(true))
        .arg(pane_agent_state_option("state"))
        .arg(option("message", "TEXT"))
        .arg(option("seq", "N"))
        .arg(option("agent-session-id", "ID"))
        .arg(path_option("agent-session-path", "PATH"))
}

fn report_agent_session_command() -> Command {
    Command::new("report-agent-session")
        .about(t!("cli.pane_report_agent_session_about").to_string())
        .arg(required("pane_id", "PANE_ID"))
        .arg(option("source", "ID").required(true))
        .arg(option("agent", "LABEL").required(true))
        .arg(option("seq", "N"))
        .arg(option("agent-session-id", "ID"))
        .arg(path_option("agent-session-path", "PATH"))
        .arg(option("session-start-source", "SOURCE"))
}

fn release_agent_command() -> Command {
    Command::new("release-agent")
        .about(t!("cli.pane_release_agent_about").to_string())
        .arg(required("pane_id", "PANE_ID"))
        .arg(option("source", "ID").required(true))
        .arg(option("agent", "LABEL").required(true))
        .arg(option("seq", "N"))
}

fn report_metadata_command() -> Command {
    Command::new("report-metadata")
        .about(t!("cli.pane_report_metadata_about").to_string())
        .arg(required("pane_id", "PANE_ID"))
        .arg(option("source", "ID").required(true))
        .arg(option("agent", "LABEL"))
        .arg(option("applies-to-source", "ID"))
        .arg(option("title", "TEXT"))
        .arg(flag("clear-title"))
        .arg(option("display-agent", "TEXT"))
        .arg(flag("clear-display-agent"))
        .arg(option("state-label", "STATUS=TEXT"))
        .arg(flag("clear-state-labels"))
        .arg(repeatable_option("token", "NAME=VALUE"))
        .arg(repeatable_option("clear-token", "NAME"))
        .arg(option("seq", "N"))
        .arg(option("ttl-ms", "N"))
}

fn terminal_command() -> Command {
    Command::new("terminal")
        .about(t!("cli.terminal_about").to_string())
        .subcommand(
            Command::new("attach")
                .about(t!("cli.terminal_attach_about").to_string())
                .arg(required("terminal_id", "TERMINAL_ID"))
                .arg(flag("takeover")),
        )
        .subcommand(
            Command::new("session")
                .about(t!("cli.terminal_session_about").to_string())
                .subcommand(
                    Command::new("control")
                        .about(t!("cli.terminal_control_about").to_string())
                        .arg(required("target", "TARGET"))
                        .arg(flag("takeover"))
                        .arg(option("cols", "N"))
                        .arg(option("rows", "N")),
                )
                .subcommand(
                    Command::new("observe")
                        .about(t!("cli.terminal_observe_about").to_string())
                        .arg(required("target", "TARGET"))
                        .arg(option("cols", "N"))
                        .arg(option("rows", "N")),
                ),
        )
        .subcommand(
            Command::new("title")
                .about(t!("cli.terminal_title_about").to_string())
                .subcommand(
                    Command::new("set")
                        .about(t!("cli.terminal_set_about").to_string())
                        .arg(required("title", "TITLE")),
                )
                .subcommand(
                    Command::new("clear").about(t!("cli.terminal_clear_about").to_string()),
                ),
        )
}

fn session_command() -> Command {
    Command::new("session")
        .about(t!("cli.session_about").to_string())
        .subcommand(
            Command::new("list")
                .about(t!("cli.session_list_about").to_string())
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("attach")
                .about(t!("cli.session_attach_about").to_string())
                .arg(required("name", "NAME")),
        )
        .subcommand(
            Command::new("stop")
                .about(t!("cli.session_stop_about").to_string())
                .arg(required("name", "NAME"))
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("delete")
                .about(t!("cli.session_delete_about").to_string())
                .arg(required("name", "NAME"))
                .arg(json_flag()),
        )
}

fn integration_command() -> Command {
    Command::new("integration")
        .about(t!("cli.integration_about").to_string())
        .subcommand(
            Command::new("install")
                .about(t!("cli.integration_install_about").to_string())
                .arg(integration_target_arg()),
        )
        .subcommand(
            Command::new("uninstall")
                .about(t!("cli.integration_uninstall_about").to_string())
                .arg(integration_target_arg()),
        )
        .subcommand(
            Command::new("status")
                .about(t!("cli.integration_status_about").to_string())
                .arg(flag("outdated-only")),
        )
}

fn plugin_command() -> Command {
    Command::new("plugin")
        .about(t!("cli.plugin_about").to_string())
        .subcommand(
            Command::new("install")
                .about(t!("cli.plugin_install_about").to_string())
                .arg(required("source", "OWNER/REPO[/SUBDIR]"))
                .arg(option("ref", "REF"))
                .arg(
                    Arg::new("yes")
                        .short('y')
                        .long("yes")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("uninstall")
                .about(t!("cli.plugin_uninstall_about").to_string())
                .arg(required("plugin", "PLUGIN")),
        )
        .subcommand(
            Command::new("link")
                .about(t!("cli.plugin_link_about").to_string())
                .arg(path_arg("path", "PATH"))
                .arg(flag("disabled"))
                .arg(flag("enabled")),
        )
        .subcommand(
            Command::new("unlink")
                .about(t!("cli.plugin_unlink_about").to_string())
                .arg(required("plugin_id", "PLUGIN_ID")),
        )
        .subcommand(
            Command::new("enable")
                .about(t!("cli.plugin_enable_about").to_string())
                .arg(required("plugin_id", "PLUGIN_ID")),
        )
        .subcommand(
            Command::new("disable")
                .about(t!("cli.plugin_disable_about").to_string())
                .arg(required("plugin_id", "PLUGIN_ID")),
        )
        .subcommand(
            Command::new("list")
                .about(t!("cli.plugin_list_about").to_string())
                .arg(option("plugin", "ID"))
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("config-dir")
                .about(t!("cli.plugin_config_dir_about").to_string())
                .arg(required("plugin_id", "PLUGIN_ID")),
        )
        .subcommand(
            Command::new("action")
                .about(t!("cli.plugin_action_about").to_string())
                .subcommand(
                    Command::new("list")
                        .about(t!("cli.plugin_action_list_about").to_string())
                        .arg(option("plugin", "ID")),
                )
                .subcommand(
                    Command::new("invoke")
                        .about(t!("cli.plugin_action_invoke_about").to_string())
                        .arg(required("action_id", "ACTION_ID"))
                        .arg(option("plugin", "ID")),
                ),
        )
        .subcommand(
            Command::new("log")
                .about(t!("cli.plugin_log_about").to_string())
                .visible_alias("logs")
                .subcommand(
                    Command::new("list")
                        .about(t!("cli.plugin_log_list_about").to_string())
                        .arg(option("plugin", "ID"))
                        .arg(option("limit", "N")),
                ),
        )
        .subcommand(
            Command::new("pane")
                .about(t!("cli.plugin_pane_about").to_string())
                .subcommand(
                    Command::new("open")
                        .about(t!("cli.plugin_pane_open_about").to_string())
                        .arg(option("plugin", "ID"))
                        .arg(option("entrypoint", "ID"))
                        .arg(
                            option("placement", "PLACEMENT")
                                .value_parser(["overlay", "split", "tab", "zoomed"]),
                        )
                        .arg(option("workspace", "ID"))
                        .arg(option("target-pane", "PANE"))
                        .arg(split_direction_option())
                        .arg(path_option("cwd", "PATH"))
                        .arg(env_option())
                        .arg(flag("focus"))
                        .arg(flag("no-focus")),
                )
                .subcommand(
                    Command::new("focus")
                        .about(t!("cli.plugin_pane_focus_about").to_string())
                        .arg(required("pane_id", "PANE_ID")),
                )
                .subcommand(
                    Command::new("close")
                        .about(t!("cli.plugin_pane_close_about").to_string())
                        .arg(required("pane_id", "PANE_ID")),
                ),
        )
}

fn current_pane_args() -> [Arg; 2] {
    [option("pane", "ID"), flag("current")]
}

fn integration_target_arg() -> Arg {
    Arg::new("target")
        .value_name("TARGET")
        .required(true)
        .value_parser(integration_target_values())
}

fn integration_target_values() -> Vec<&'static str> {
    crate::api::schema::IntegrationTarget::ALL
        .into_iter()
        .map(crate::integration::integration_target_label)
        .collect()
}

fn id_command(name: &'static str, id: &'static str, about: String) -> Command {
    Command::new(name).about(about).arg(required(id, id))
}

fn direction_option() -> Arg {
    option("direction", "DIRECTION").value_parser(["left", "right", "up", "down"])
}

fn required_direction_option() -> Arg {
    direction_option().required(true)
}

fn split_direction_option() -> Arg {
    option("direction", "DIRECTION").value_parser(["right", "down"])
}

fn pane_agent_state_option(name: &'static str) -> Arg {
    option(name, "STATUS")
        .required(true)
        .value_parser(["idle", "working", "blocked", "unknown"])
}

fn read_source_option(include_detection: bool) -> Arg {
    let values = if include_detection {
        vec!["visible", "recent", "recent-unwrapped", "detection"]
    } else {
        vec!["visible", "recent", "recent-unwrapped"]
    };
    option("source", "SOURCE")
        .value_parser(values)
        .help("Terminal snapshot source (default: recent)")
}

fn text_ansi_format_option() -> Arg {
    option("format", "FORMAT").value_parser(["text", "ansi"])
}

fn text_json_format_option() -> Arg {
    option("format", "FORMAT").value_parser(["text", "json"])
}

fn json_flag() -> Arg {
    flag("json")
}

fn help_flag() -> Arg {
    Arg::new("help")
        .short('h')
        .long("help")
        .action(ArgAction::SetTrue)
        .help(t!("cli.help_help").to_string())
}

fn env_option() -> Arg {
    option("env", "KEY=VALUE")
        .action(ArgAction::Append)
        .help(t!("cli.env_help").to_string())
}

fn flag(name: &'static str) -> Arg {
    Arg::new(name).long(name).action(ArgAction::SetTrue)
}

fn option(name: &'static str, value_name: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .value_name(value_name)
        .action(ArgAction::Set)
}

fn repeatable_option(name: &'static str, value_name: &'static str) -> Arg {
    option(name, value_name).action(ArgAction::Append)
}

fn path_option(name: &'static str, value_name: &'static str) -> Arg {
    option(name, value_name).value_hint(ValueHint::AnyPath)
}

fn required(name: &'static str, value_name: &'static str) -> Arg {
    Arg::new(name).value_name(value_name).required(true)
}

fn path_arg(name: &'static str, value_name: &'static str) -> Arg {
    required(name, value_name).value_hint(ValueHint::AnyPath)
}

#[cfg(test)]
mod tests {
    use clap::{Arg, Command};
    use rust_i18n::t;

    fn command_path<'a>(cmd: &'a Command, path: &[&str]) -> &'a Command {
        let mut current = cmd;
        for name in path {
            current = current
                .get_subcommands()
                .find(|subcommand| subcommand.get_name() == *name)
                .unwrap_or_else(|| panic!("missing command path segment {name}"));
        }
        current
    }

    fn option_values(cmd: &Command, option: &str) -> Vec<String> {
        let arg = cmd
            .get_arguments()
            .find(|arg| arg.get_long() == Some(option))
            .unwrap_or_else(|| panic!("missing --{option}"));
        arg.get_value_parser()
            .possible_values()
            .into_iter()
            .flatten()
            .map(|value| value.get_name().to_string())
            .collect()
    }

    fn has_option(cmd: &Command, option: &str) -> bool {
        cmd.get_arguments()
            .any(|arg| arg.get_long() == Some(option))
    }

    fn option_arg<'a>(cmd: &'a Command, option: &str) -> &'a Arg {
        cmd.get_arguments()
            .find(|arg| arg.get_long() == Some(option))
            .unwrap_or_else(|| panic!("missing --{option}"))
    }

    fn argument<'a>(cmd: &'a Command, id: &str) -> &'a Arg {
        cmd.get_arguments()
            .find(|arg| arg.get_id() == id)
            .unwrap_or_else(|| panic!("missing argument {id}"))
    }

    fn collect_subcommand_paths(
        cmd: &Command,
        path: &mut Vec<String>,
        paths: &mut Vec<Vec<String>>,
    ) {
        for subcommand in cmd.get_subcommands() {
            path.push(subcommand.get_name().to_string());
            paths.push(path.clone());
            collect_subcommand_paths(subcommand, path, paths);
            path.pop();
        }
    }

    fn assert_command_descriptions(cmd: &Command, path: &mut Vec<String>) {
        if !path.is_empty() {
            assert!(
                cmd.get_about().is_some(),
                "missing completion description for {}",
                path.join(" ")
            );
        }
        for subcommand in cmd.get_subcommands() {
            path.push(subcommand.get_name().to_string());
            assert_command_descriptions(subcommand, path);
            path.pop();
        }
    }

    #[test]
    fn spec_describes_all_completion_commands() {
        let cmd = super::command();
        assert_command_descriptions(&cmd, &mut Vec::new());
    }

    #[test]
    fn spec_passes_clap_invariants() {
        super::command().debug_assert();
    }

    #[test]
    fn every_spec_subcommand_renders_short_and_long_help() {
        let mut paths = Vec::new();
        collect_subcommand_paths(&super::command(), &mut Vec::new(), &mut paths);

        for path in paths {
            for flag in ["-h", "--help"] {
                let mut args = vec!["herdr".to_string()];
                args.extend(path.iter().cloned());
                args.push(flag.to_string());
                let mut output = Vec::new();
                assert!(
                    super::write_requested_help(&args, &mut output, || {}).unwrap(),
                    "help was not handled for herdr {} {flag}",
                    path.join(" ")
                );
                let output = String::from_utf8(output).unwrap();
                assert!(
                    output.contains(&format!("Usage: herdr {}", path.join(" "))),
                    "unexpected help for herdr {}: {output}",
                    path.join(" ")
                );
            }
        }
    }

    #[test]
    fn spec_includes_completion_alias_and_shells() {
        let cmd = super::command();
        let completion = command_path(&cmd, &["completion"]);
        assert!(completion
            .get_all_aliases()
            .any(|alias| alias == "completions"));
        let shells = completion
            .get_arguments()
            .find(|arg| arg.get_id() == "shell")
            .unwrap()
            .get_value_parser()
            .possible_values()
            .unwrap()
            .map(|value| value.get_name().to_string())
            .collect::<Vec<_>>();
        assert!(shells.contains(&"zsh".to_string()));
        assert!(shells.contains(&"fish".to_string()));
    }

    #[test]
    fn spec_matches_all_integration_targets() {
        let cmd = super::command();
        let install = command_path(&cmd, &["integration", "install"]);
        assert_eq!(
            argument(install, "target")
                .get_value_parser()
                .possible_values()
                .unwrap()
                .map(|value| value.get_name().to_string())
                .collect::<Vec<_>>(),
            crate::api::schema::IntegrationTarget::ALL
                .map(crate::integration::integration_target_label)
                .map(str::to_string)
        );
    }

    #[test]
    fn spec_marks_runtime_required_options_as_required() {
        for (path, options) in [
            (&["workspace", "report-metadata"][..], &["source"][..]),
            (&["pane", "neighbor"][..], &["direction"][..]),
            (&["pane", "focus"][..], &["direction"][..]),
            (&["pane", "resize"][..], &["direction"][..]),
            (&["pane", "report-agent"][..], &["source", "agent"][..]),
            (
                &["pane", "report-agent-session"][..],
                &["source", "agent"][..],
            ),
            (&["pane", "release-agent"][..], &["source", "agent"][..]),
            (&["pane", "report-metadata"][..], &["source"][..]),
        ] {
            let cmd = command_path(&super::command(), path).clone();
            for option in options {
                assert!(
                    option_arg(&cmd, option).is_required_set(),
                    "herdr {} --{option} should be required",
                    path.join(" ")
                );
            }
        }
    }

    #[test]
    fn agent_prompt_until_requires_wait() {
        let error = super::command()
            .try_get_matches_from([
                "herdr", "agent", "prompt", "reviewer", "hello", "--until", "idle",
            ])
            .unwrap_err();
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn agent_rename_requires_exactly_one_name_or_clear() {
        for valid in [
            &["herdr", "agent", "rename", "reviewer", "worker"][..],
            &["herdr", "agent", "rename", "reviewer", "--clear"][..],
        ] {
            assert!(super::command().try_get_matches_from(valid).is_ok());
        }
        for invalid in [
            &["herdr", "agent", "rename", "reviewer"][..],
            &["herdr", "agent", "rename", "reviewer", "worker", "--clear"][..],
        ] {
            assert!(super::command().try_get_matches_from(invalid).is_err());
        }

        let mut help = Vec::new();
        super::write_requested_help(
            &[
                "herdr".to_string(),
                "agent".to_string(),
                "rename".to_string(),
                "--help".to_string(),
            ],
            &mut help,
            || {},
        )
        .unwrap();
        assert!(String::from_utf8(help)
            .unwrap()
            .contains("Usage: herdr agent rename <TARGET> <NAME>|--clear"));
    }

    #[test]
    fn worktree_json_compatibility_flag_stays_out_of_public_spec() {
        let cmd = super::command();
        for subcommand in ["list", "create", "open", "remove"] {
            let worktree_command = command_path(&cmd, &["worktree", subcommand]);
            assert!(
                !has_option(worktree_command, "json"),
                "herdr worktree {subcommand} should not advertise --json"
            );
        }
    }

    #[test]
    fn spec_includes_nested_plugin_pane_open_options() {
        let cmd = super::command();
        let open = command_path(&cmd, &["plugin", "pane", "open"]);
        assert!(open
            .get_arguments()
            .any(|arg| arg.get_long() == Some("entrypoint")));
        assert!(option_values(open, "placement").contains(&"zoomed".to_string()));
    }

    #[test]
    fn spec_keeps_agent_wait_status_free() {
        let cmd = super::command();
        let wait = command_path(&cmd, &["agent", "wait"]);
        assert!(!has_option(wait, "status"));
        assert_eq!(
            option_values(wait, "until"),
            ["idle", "working", "blocked", "done", "unknown"]
        );
        assert!(has_option(wait, "timeout"));
    }

    #[test]
    fn spec_matches_refactored_agent_and_pane_commands() {
        let cmd = super::command();
        assert!(cmd
            .get_subcommands()
            .all(|subcommand| subcommand.get_name() != "wait"));

        let agent = command_path(&cmd, &["agent"]);
        assert!(agent
            .get_subcommands()
            .any(|subcommand| subcommand.get_name() == "send-keys"));
        assert!(agent
            .get_subcommands()
            .any(|subcommand| subcommand.get_name() == "wait"));
        assert!(agent
            .get_subcommands()
            .all(|subcommand| subcommand.get_name() != "send"));

        let pane = command_path(&cmd, &["pane"]);
        assert!(pane
            .get_subcommands()
            .any(|subcommand| subcommand.get_name() == "wait-output"));
    }

    #[test]
    fn spec_includes_pane_read_raw_flag() {
        let cmd = super::command();
        let pane_read = command_path(&cmd, &["pane", "read"]);
        assert!(has_option(pane_read, "raw"));
    }

    #[test]
    fn spec_matches_pane_split_direction_flag() {
        let cmd = super::command();
        let pane_split = command_path(&cmd, &["pane", "split"]);
        assert!(has_option(pane_split, "direction"));
        assert!(!has_option(pane_split, "split"));
        assert_eq!(option_values(pane_split, "direction"), ["right", "down"]);
    }

    #[test]
    fn spec_models_agent_start_target_and_trailing_args() {
        let cmd = super::command();
        let agent_start = command_path(&cmd, &["agent", "start"]);
        assert!(has_option(agent_start, "kind"));
        assert_eq!(
            option_values(agent_start, "kind"),
            crate::detect::Agent::ALL
                .map(crate::detect::agent_label)
                .map(str::to_string)
        );
        assert!(has_option(agent_start, "pane"));
        for legacy in ["cwd", "workspace", "tab", "split", "focus", "env", "argv"] {
            assert!(!has_option(agent_start, legacy), "legacy option --{legacy}");
        }
        assert!(agent_start
            .get_arguments()
            .any(|arg| arg.get_id() == "agent_args"));
    }

    fn long_help(path: &[&str]) -> String {
        let mut args = vec!["herdr".to_string()];
        args.extend(path.iter().map(|segment| segment.to_string()));
        args.push("--help".to_string());
        let mut output = Vec::new();
        assert!(
            super::write_requested_help(&args, &mut output, || {}).unwrap(),
            "help was not handled for herdr {}",
            path.join(" ")
        );
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn agent_resources_appear_on_command_groups_but_not_leaf_commands() {
        for group in ["agent", "pane", "workspace", "terminal"] {
            let help = long_help(&[group]);
            assert!(
                help.contains(super::super::AGENT_HELP_FOOTER),
                "herdr {group} is missing agent resources: {help}"
            );
        }

        let leaf = long_help(&["agent", "wait"]);
        assert!(
            !leaf.contains(super::super::AGENT_HELP_FOOTER),
            "leaf help should stay focused: {leaf}"
        );
    }

    #[test]
    fn next_step_hints_render_without_replacing_existing_after_help() {
        let agent_start = long_help(&["agent", "start"]);
        assert!(
            agent_start.contains("The pane must be at its interactive shell prompt."),
            "agent start dropped its existing after_help: {agent_start}"
        );
        assert!(
            agent_start.contains("next: herdr agent prompt <TARGET> <TEXT> --wait"),
            "agent start is missing its next-step hint: {agent_start}"
        );

        let pane_send_text = long_help(&["pane", "send-text"]);
        assert!(
            pane_send_text.contains(
                "next: herdr pane run <PANE_ID> <COMMAND> sends text and Enter in one call"
            ),
            "pane send-text is missing its next-step hint: {pane_send_text}"
        );
    }

    #[test]
    fn completion_generation_succeeds_for_every_supported_shell() {
        for shell in [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Elvish,
            clap_complete::Shell::Fish,
            clap_complete::Shell::PowerShell,
            clap_complete::Shell::Zsh,
        ] {
            let mut cmd = super::command();
            let mut output = Vec::new();
            clap_complete::generate(shell, &mut cmd, "herdr", &mut output);
            assert!(!output.is_empty(), "empty {shell:?} completion output");
        }
    }
}
