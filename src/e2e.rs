//! End-to-end option tests: for every option, the window control and the
//! flag reach the same `launcher.cfg` key, and that key reaches what the
//! game sees (dgVoodoo.conf, Xidi, the registry, the environment, the
//! patch set) through `plan::plan`, the function a launch carries out.
//! Headless (`egui_kittest`): on the host and under `scripts/wine-test`.

use std::fs;
use std::path::{Path, PathBuf};

use rea_launcher::pe::{Format, synthetic};
use rea_launcher_ui::egui::Role;
use rea_launcher_ui::testing::{self, Harness, Queryable};

use crate::components::Platform;
use crate::config::{self, Resolution, Settings};
use crate::form::Tab;
use crate::plan::{self, Plan};
use crate::registry::RegValue;
use crate::window::{Host, METRICS, PALETTE, Window};

const UHD: Option<(u32, u32)> = Some((3840, 2160));

/// Removes a test's scratch folder when the test ends.
struct Clean(PathBuf);

impl Drop for Clean {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A scratch folder with a hostile name (space, apostrophe, accents),
/// under the temp folder only; removed when the guard drops.
fn scratch(name: &str) -> (PathBuf, Clean) {
    let root = std::env::temp_dir().join(format!("cmr4-e2e-{}-{name}", std::process::id()));
    let dir = root.join("Jogos d'Ação José");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&dir).unwrap();
    (dir, Clean(root))
}

/// A game folder whose `CMR4.EXE` is a tiny synthetic PE32 (no game
/// bytes): an unknown build, which the window accepts.
fn fake_game(dir: &Path) -> PathBuf {
    let game = dir.join("Colin McRae Rally 04");
    fs::create_dir_all(&game).unwrap();
    let exe = synthetic::image(
        Format::Pe32,
        0x40_0000,
        &[(".text", 0x1000, &[0xc3; 16], synthetic::CODE)],
    );
    fs::write(game.join("CMR4.EXE"), exe).unwrap();
    game
}

fn host() -> Host {
    Host {
        screen: UHD,
        linux: false,
        pads: 1,
        pick_folder: |_| None,
        open_url: |_| {},
    }
}

/// The window over `cfg` (written to a settings file) with a game folder.
fn open(dir: &Path, cfg: &str) -> (Harness<'static, Window>, PathBuf) {
    let game = fake_game(dir);
    let path = dir.join(config::FILE_NAME);
    let text = format!("game = {}\n{cfg}", game.display());
    fs::write(&path, &text).unwrap();
    let window = Window::new(path.clone(), Some(&text), host());
    let ui = testing::harness(&PALETTE, &METRICS, window, |ui, window| window.draw(ui));
    (ui, path)
}

fn tab(ui: &mut Harness<'static, Window>, name: &str) {
    ui.get_by_role_and_label(Role::Tab, name).click();
    ui.run();
}

/// Presses Play and returns the saved `launcher.cfg`.
fn play(ui: &mut Harness<'static, Window>, path: &Path) -> String {
    ui.get_by_role_and_label(Role::Button, "Play").click();
    ui.run();
    assert!(ui.state().played, "Play didn't save");
    fs::read_to_string(path).unwrap()
}

fn has_line(text: &str, line: &str) -> bool {
    text.lines().any(|l| l.trim() == line)
}

/// What a launch with this file and these flags would do on a 4K screen
/// under Wine, for a known build (the fix applies when wanted).
fn launch(cfg: &str, flags: &[&str]) -> Plan {
    let cli = config::parse_args(flags.iter().map(|f| f.to_string())).unwrap();
    let (file, warnings) = Settings::parse_file(cfg);
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut settings = file
        .resolve_look(UHD)
        .merged(cli.settings.resolve_look(UHD));
    settings.resolution = settings
        .resolution
        .or(Some(config::default_resolution(UHD)));
    let runtime = settings
        .runtime
        .clone()
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\José\AppData\Local\CMR4Launcher\CMR4"));
    plan::plan(
        &settings,
        settings.widescreen(),
        &runtime,
        &Platform::new(true, false, None),
        None,
        None,
    )
}

fn file(plan: &Plan, name: &str) -> String {
    plan.files
        .iter()
        .find(|(n, _)| *n == name)
        .and_then(|(_, text)| text.clone())
        .unwrap_or_default()
}

fn dgvoodoo(plan: &Plan, key: &str) -> String {
    file(plan, "dgVoodoo.conf")
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().to_owned())
        })
        .unwrap_or_default()
}

fn reg(plan: &Plan, name: &str) -> RegValue {
    plan.registry
        .iter()
        .find(|w| w.name == name)
        .unwrap()
        .value
        .clone()
}

/// The plan from a saved file equals the plan from the same value as a
/// flag over an empty file.
fn same_as_flag(saved: &str, flags: &[&str]) -> Plan {
    let from_file = launch(saved, &[]);
    let from_flag = launch("", flags);
    assert_eq!(from_file, from_flag, "file and {flags:?} differ");
    from_file
}

/// The saved file without the game line (the flags don't set it).
fn picture_only(saved: &str) -> String {
    saved
        .lines()
        .filter(|l| !l.starts_with("game ="))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn resolution_chip_and_flag_reach_the_registry_dgvoodoo_and_the_patch() {
    let (dir, _clean) = scratch("resolution");
    let (mut ui, path) = open(&dir, "");
    tab(&mut ui, "Picture");
    let row = ui.get_by_role_and_label(Role::RadioGroup, "Resolution");
    assert!(testing::description(&row).unwrap().contains("16:9"));
    ui.get_by_role_and_label(Role::RadioButton, "2560×1440")
        .click();
    ui.run();
    assert!(
        ui.query_by_label_contains("full 16:9 widescreen view")
            .is_some()
    );
    let saved = play(&mut ui, &path);
    assert!(has_line(&saved, "resolution = 2560x1440"), "{saved}");
    let plan = same_as_flag(
        &picture_only(&saved),
        &[
            "--resolution",
            "2560x1440",
            "--fps-limit",
            "off",
            "--vsync",
            "off",
            "--controller",
            "on",
        ],
    );
    assert_eq!(plan.patch_sets, ["widescreen"]);
    assert_eq!(plan.game_mode, Resolution::Custom(1920, 1440));
    assert_eq!(reg(&plan, "Width"), RegValue::Dword(1920));
    assert_eq!(reg(&plan, "Height"), RegValue::Dword(1440));
    assert_eq!(dgvoodoo(&plan, "Resolution"), "h:2560, v:1440");
    assert_eq!(dgvoodoo(&plan, "ScalingMode"), "stretched");

    // A 4:3 size: the game's own view, no patch, native mode.
    let (mut ui, path) = open(&dir, "");
    tab(&mut ui, "Picture");
    ui.get_by_role_and_label(Role::RadioButton, "1600×1200")
        .click();
    ui.run();
    assert!(
        ui.query_by_label_contains("4:3: you'll see the game's own view")
            .is_some()
    );
    let saved = play(&mut ui, &path);
    assert!(has_line(&saved, "resolution = 1600x1200"));
    let plan = launch(&saved.replace("game =", "#game ="), &[]);
    assert!(plan.patch_sets.is_empty());
    assert_eq!(reg(&plan, "Width"), RegValue::Dword(1600));
    assert_eq!(dgvoodoo(&plan, "ScalingMode"), "stretched_ar");

    // Original keeps the game's own mode (seeded 1024x768 when absent).
    let plan = launch("", &["--resolution", "original"]);
    assert!(plan.patch_sets.is_empty());
    assert_eq!(dgvoodoo(&plan, "Resolution"), "unforced");
    assert_eq!(reg(&plan, "Width"), RegValue::Dword(1024));
}

#[test]
fn frame_rate_cap_chip_and_flag_reach_dgvoodoo() {
    let (dir, _clean) = scratch("fps");
    let (mut ui, path) = open(&dir, "");
    tab(&mut ui, "Picture");
    ui.get_by_role_and_label(Role::RadioGroup, "Frame-rate cap");
    ui.get_by_role_and_label(Role::RadioButton, "60").click();
    ui.run();
    let saved = play(&mut ui, &path);
    assert!(has_line(&saved, "fps_limit = 60"), "{saved}");
    assert_eq!(dgvoodoo(&launch(&saved, &[]), "FPSLimit"), "60");
    assert_eq!(
        dgvoodoo(&launch("", &["--fps-limit", "60"]), "FPSLimit"),
        "60"
    );

    let (mut ui, path) = open(&dir, "fps_limit = 90\n");
    tab(&mut ui, "Picture");
    // A saved value outside the list is shown and kept.
    assert!(testing::checked(&ui.get_by_label("90 (saved)")));
    ui.get_by_role_and_label(Role::RadioButton, "No cap")
        .click();
    ui.run();
    let saved = play(&mut ui, &path);
    assert!(has_line(&saved, "fps_limit = off"), "{saved}");
    assert_eq!(dgvoodoo(&launch(&saved, &[]), "FPSLimit"), "0");
    assert_eq!(
        dgvoodoo(&launch("", &["--fps-limit", "0"]), "FPSLimit"),
        "0"
    );
}

#[test]
fn vsync_chip_and_flag_reach_dgvoodoo() {
    let (dir, _clean) = scratch("vsync");
    let (mut ui, path) = open(&dir, "");
    tab(&mut ui, "Picture");
    let row = ui.get_by_role_and_label(Role::RadioGroup, "VSync");
    assert!(testing::description(&row).unwrap().contains("tearing"));
    ui.get_by_role_and_label(Role::RadioButton, "On").click();
    ui.run();
    let saved = play(&mut ui, &path);
    assert!(has_line(&saved, "vsync = on"));
    assert_eq!(dgvoodoo(&launch(&saved, &[]), "ForceVerticalSync"), "true");
    assert_eq!(
        dgvoodoo(&launch("", &["--vsync", "on"]), "ForceVerticalSync"),
        "true"
    );
    assert_eq!(
        dgvoodoo(&launch("", &["--vsync", "off"]), "ForceVerticalSync"),
        "false"
    );
}

#[test]
fn gamepad_switch_and_flag_reach_xidi_the_environment_and_winebus() {
    let (dir, _clean) = scratch("controller");
    let (mut ui, path) = open(&dir, "");
    tab(&mut ui, "Controls");
    let switch = ui.get_by_role_and_label(Role::Switch, "Gamepad");
    assert!(testing::checked(&switch));
    assert!(
        testing::description(&switch)
            .unwrap()
            .contains("left stick steers")
    );
    switch.click();
    ui.run();
    let saved = play(&mut ui, &path);
    assert!(has_line(&saved, "controller = off"));
    let off = launch(&saved, &[]);
    assert_eq!(off, launch("", &["--controller", "off"]));
    assert!(!off.wrappers.contains(&"dinput8.dll"));
    assert!(off.taken_out.contains(&"dinput8.dll"));
    assert!(off.files.contains(&("Xidi.ini", None)));
    assert!(off.winebus.is_empty());
    assert!(!off.gamepad_profile);
    assert_eq!(off.env[0].1, "dsound,d3d9,d3d11,dxgi=n,b;mscoree=d");

    let on = launch("", &["--controller", "on"]);
    assert!(on.wrappers.contains(&"dinput8.dll"));
    assert!(file(&on, "Xidi.ini").contains("TriggerRT = Axis(Y, -)"));
    assert_eq!(on.winebus, [("Map Controllers", 1)]);
    assert_eq!(on.env[0].1, "dinput8,dsound,d3d9,d3d11,dxgi=n,b;mscoree=d");
}

#[test]
fn look_cards_and_the_look_flag_set_the_same_values() {
    let (dir, _clean) = scratch("look");
    for (card, flag) in [
        ("Original", "original"),
        ("Widescreen", "widescreen"),
        ("Steam Deck", "steam_deck"),
    ] {
        let (mut ui, path) = open(&dir, "resolution = 1600x1200\nfps_limit = 144\n");
        let node = ui.get_by_label(card);
        assert!(testing::description(&node).is_some(), "{card} has no note");
        node.click();
        ui.run();
        assert!(testing::checked(&ui.get_by_label(card)), "{card}");
        let saved = play(&mut ui, &path);
        let from_window = launch(&picture_only(&saved), &[]);
        let from_flag = launch(
            "resolution = 1600x1200\nfps_limit = 144\n",
            &["--look", flag],
        );
        assert_eq!(from_window, from_flag, "{card}");
    }
    let deck = launch("", &["--look", "steam_deck"]);
    assert_eq!(dgvoodoo(&deck, "Resolution"), "h:1280, v:800");
    assert_eq!(dgvoodoo(&deck, "FPSLimit"), "60");
    assert!(deck.patch_sets.is_empty());
    // Custom opens the Picture tab and keeps the values.
    let (mut ui, _) = open(&dir, "resolution = 1600x1200\n");
    ui.get_by_label("Custom").click();
    ui.run();
    assert_eq!(ui.state().tab, Tab::Picture);
    assert_eq!(ui.state().form.resolution, Resolution::Custom(1600, 1200));
}

#[test]
fn game_and_runtime_folders_typed_in_the_window_are_saved_and_used() {
    let (dir, _clean) = scratch("folders");
    let (mut ui, path) = open(&dir, "");
    let runtime = dir.join("Cópia privada");
    let field = ui.get_by_role_and_label(Role::TextInput, "Private copy folder");
    field.click();
    ui.run();
    ui.get_by_role_and_label(Role::TextInput, "Private copy folder")
        .type_text(&runtime.display().to_string());
    ui.run();
    let saved = play(&mut ui, &path);
    let (settings, warnings) = Settings::parse_file(&saved);
    assert!(warnings.is_empty());
    assert_eq!(settings.runtime.as_deref(), Some(runtime.as_path()));
    assert!(settings.game.unwrap().ends_with("Colin McRae Rally 04"));
    let plan = launch(&saved, &[]);
    assert_eq!(
        reg(&plan, "INSTALL_PATH"),
        RegValue::String(runtime.display().to_string())
    );
    let flag = launch("", &["--runtime", &runtime.display().to_string()]);
    assert_eq!(reg(&flag, "INSTALL_PATH"), reg(&plan, "INSTALL_PATH"));
    let cli = config::parse_args(["--game".into(), r"D:\Jogos d'Ação\CMR04".into()]).unwrap();
    assert_eq!(
        cli.settings.game.as_deref(),
        Some(Path::new(r"D:\Jogos d'Ação\CMR04"))
    );
}

#[test]
fn play_waits_for_the_game_folder_and_says_why() {
    let (dir, _clean) = scratch("play");
    let path = dir.join(config::FILE_NAME);
    let mut ui = testing::harness(
        &PALETTE,
        &METRICS,
        Window::new(path.clone(), None, host()),
        |ui, window| window.draw(ui),
    );
    let play_button = ui.get_by_role_and_label(Role::Button, "Play");
    assert!(testing::disabled(&play_button));
    assert_eq!(
        testing::description(&play_button).as_deref(),
        Some("Play needs the game folder first.")
    );
    play_button.click();
    ui.run();
    assert!(!ui.state().played && !path.exists());
    // A folder without cmr4.exe says so; a game folder makes Play ready.
    ui.state_mut().form.game = dir.display().to_string();
    ui.run();
    ui.run();
    assert!(ui.query_by_label_contains("There is no cmr4.exe").is_some());
    ui.state_mut().form.game = fake_game(&dir).display().to_string();
    ui.run();
    ui.run();
    // The synthetic exe is an unknown build: playable, with the report link.
    assert!(
        ui.query_by_label_contains("issues/new?template=unknown-edition.yml")
            .is_some()
    );
    assert!(!testing::disabled(
        &ui.get_by_role_and_label(Role::Button, "Play")
    ));
    let saved = play(&mut ui, &path);
    // First run: the template, the game folder, the recommended look's values.
    assert!(saved.starts_with("# Colin McRae Rally 04 launcher settings."));
    assert!(has_line(&saved, "resolution = 3840x2160"));
}

#[test]
fn every_tab_renders_and_its_controls_are_reachable_and_named() {
    let (dir, _clean) = scratch("tabs");
    let (mut ui, _) = open(&dir, "");
    for name in ["Original", "Widescreen", "Steam Deck", "Custom"] {
        assert!(
            testing::description(&ui.get_by_label(name)).is_some(),
            "{name}"
        );
    }
    assert!(
        ui.query_by_role_and_label(Role::TextInput, "Game folder")
            .is_some()
    );
    assert_eq!(
        ui.get_all_by_role_and_label(Role::Button, "Browse…")
            .count(),
        2
    );
    // The recommended look carries the badge in its description.
    assert!(
        testing::description(&ui.get_by_label("Widescreen"))
            .unwrap()
            .contains("Recommended")
    );
    tab(&mut ui, "Picture");
    for row in ["Resolution", "Frame-rate cap", "VSync"] {
        let node = ui.get_by_role_and_label(Role::RadioGroup, row);
        assert!(testing::description(&node).is_some(), "{row}");
    }
    assert!(
        ui.query_by_label("Widescreen").is_none(),
        "no widescreen switch any more"
    );
    tab(&mut ui, "Controls");
    assert!(
        ui.query_by_role_and_label(Role::Switch, "Gamepad")
            .is_some()
    );
    // Keyboard: Ctrl+Tab and Ctrl+Shift+Tab change tabs.
    ui.key_press_modifiers(
        rea_launcher_ui::egui::Modifiers::CTRL,
        rea_launcher_ui::egui::Key::Tab,
    );
    ui.run();
    assert_eq!(ui.state().tab, Tab::Play);
    ui.key_press_modifiers(
        rea_launcher_ui::egui::Modifiers::CTRL | rea_launcher_ui::egui::Modifiers::SHIFT,
        rea_launcher_ui::egui::Key::Tab,
    );
    ui.run();
    assert_eq!(ui.state().tab, Tab::Controls);
    // Gamepad off: no mapping table, a note instead.
    ui.get_by_role_and_label(Role::Switch, "Gamepad").click();
    ui.run();
    assert!(ui.query_by_label("What the game sees").is_none());
    assert!(
        ui.query_by_label_contains("reads the pad by itself")
            .is_some()
    );
    // The pad's shoulders change tabs; Start plays.
    ui.state_mut()
        .navigate(&[rea_launcher_ui::pad::Nav::NextTab]);
    ui.run();
    assert_eq!(ui.state().tab, Tab::Play);
}

#[test]
fn keyboard_alone_picks_a_resolution() {
    let (dir, _clean) = scratch("keyboard");
    let (mut ui, _) = open(&dir, "resolution = 1920x1080\n");
    tab(&mut ui, "Picture");
    let before = ui.state().form.resolution;
    ui.key_press(rea_launcher_ui::egui::Key::Tab);
    ui.run();
    ui.key_press(rea_launcher_ui::egui::Key::ArrowRight);
    ui.run();
    assert_ne!(ui.state().form.resolution, before);
}

#[test]
fn a_v0_2_0_settings_file_opens_saves_and_plans_cleanly() {
    let (dir, _clean) = scratch("upgrade");
    let old = "# Colin McRae Rally 04 launcher settings.\r\n# Command-line options override these values and are saved.\r\n\r\nresolution = 1920x1080\r\nwidescreen = off\r\nfps_limit = off\r\nvsync = on\r\ncontroller = on\r\n";
    let (mut ui, path) = open(&dir, old);
    assert_eq!(ui.state().form.resolution, Resolution::Custom(1920, 1080));
    let saved = play(&mut ui, &path);
    assert!(saved.contains("#widescreen ="), "{saved}");
    assert!(!saved.contains("are saved."));
    let plan = launch(&saved, &[]);
    assert_eq!(plan.patch_sets, ["widescreen"]);
    assert_eq!(dgvoodoo(&plan, "ForceVerticalSync"), "true");
}

#[test]
fn hostile_game_and_runtime_paths_sync_and_resolve() {
    let (dir, _clean) = scratch("hostile");
    let game = fake_game(&dir);
    fs::create_dir_all(game.join("Data/Pistas d'Ação")).unwrap();
    fs::write(game.join("Data/Pistas d'Ação/Finlândia.big"), b"track").unwrap();
    let runtime = dir.join("Cópia do José");
    let resolved = crate::runtime::game_folder(&game, &dir).unwrap();
    rea_launcher::paths::check_separate(&resolved, &runtime).unwrap();
    let report = crate::runtime::sync(&resolved, &runtime).unwrap();
    assert_eq!(report.brought, 2);
    assert_eq!(
        fs::read(runtime.join("Data/Pistas d'Ação/Finlândia.big")).unwrap(),
        b"track"
    );
    // The registry gets the runtime as typed, without a trailing slash.
    let plan = plan::plan(
        &Settings::default(),
        true,
        &runtime,
        &Platform::default(),
        None,
        None,
    );
    assert_eq!(
        reg(&plan, "INSTALL_PATH"),
        RegValue::String(runtime.display().to_string())
    );
    assert!(plan.prefer_fast_gpu && plan.env.is_empty());
    assert_eq!(plan.taken_out, ["d3d11.dll", "dxgi.dll"]);
}

#[test]
fn the_plan_keeps_first_run_defaults_and_the_players_overrides() {
    let runtime = Path::new(r"C:\CMR4\");
    let plan = plan::plan(
        &Settings::default(),
        true,
        runtime,
        &Platform::new(true, true, None),
        Some((800, 600)),
        Some("d3d9.dll=b;xinput1_3=n"),
    );
    assert_eq!(
        reg(&plan, "INSTALL_PATH"),
        RegValue::String(r"C:\CMR4".into())
    );
    assert_eq!(reg(&plan, "Width"), RegValue::Dword(1440));
    let seeded: Vec<_> = plan.registry.iter().filter(|w| w.if_missing).collect();
    assert_eq!(seeded.len(), 10, "LANGUAGE and the nine graphics defaults");
    assert_eq!(
        plan.env,
        [(
            "WINEDLLOVERRIDES",
            "dinput8,dsound,d3d9,d3d11,dxgi=n,b;mscoree=d;xinput1_3=n".to_string()
        )]
    );
    assert!(plan.args.is_empty());
    let text = plan.render();
    assert!(text.contains("patch sets: widescreen"));
    assert!(text.contains("LANGUAGE = string:\"E\" (if missing)"));
    assert!(text.contains("[file dgVoodoo.conf]"));
}
