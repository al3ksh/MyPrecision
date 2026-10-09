# MyPrecision — etap 1: plan implementacji

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aplikacja w zasobniku (Tauri 2) dla Dell Precision 5560, która jednym kliknięciem przełącza profile ładowania baterii i tryby chłodzenia (przez `cctk`) i pokazuje mini-podgląd czujników, w stylu Graphite.

**Architecture:** Workspace Cargo z dwoma crate'ami: `crates/core` (czysta logika: typy, parsowanie, obliczenia, historia, config — w pełni testowana jednostkowo, bez manifestu admina) oraz `src-tauri` (aplikacja: odczyty Windows/WMI/NVML, poller, tray, okna, komendy). Frontend Svelte 5 z jednym bundlem; widok wybierany po etykiecie okna (`flyout` / `main`). WebView istnieje tylko, gdy okno jest widoczne.

**Tech Stack:** Rust 1.98 (MSVC), Tauri 2.x, `tauri-plugin-single-instance`, `serde`/`serde_json`, `thiserror`, `chrono`, `wmi`, `windows` (Win32: SystemInformation, Threading, Devices::DeviceAndDriverInstallation, Devices::Properties, UI::Shell, Security), `libloading`; Svelte 5 + TypeScript + Vite, Vitest + `@testing-library/svelte` + jsdom, `svelte-check`.

**Spec:** `docs/superpowers/specs/2026-10-09-myprecision-design.md`

## Global Constraints

- **Cały interfejs aplikacji po angielsku** (decyzja użytkownika 2026-10-09, nadpisuje polskie nazwy ze spec). Liczby w formacie en-US. Nazwy profili: „Home”, „Campus”, „Storage”; profil spoza listy: „Custom (x–y%)” (półpauza `–`).
- Tryby chłodzenia (etykiety UI): Optimized = „Optimized”, Cool = „Cool”, Quiet = „Quiet”, UltraPerformance = „Ultra Performance”.
- Profile domyślne: Home = `Custom:75-80`, Campus = `Standard`, Storage = `Custom:50-60`.
- Custom: start 50–95, stop 55–100, stop − start ≥ 5. Każdy zapis musi to walidować przed wywołaniem `cctk`.
- `cctk.exe`: `C:\Program Files (x86)\Dell\Command Configure\X86_64\cctk.exe`. Wyjście: `PrimaryBattChargeCfg=<v>`, `ThermalManagement=<v>`.
- DCM: `root\dcim\sysman`, `DCIM_NumericSensor`. `CurrentReading` temperatur w pełnych °C (NIE skalować mimo `UnitModifier=-1`). CPU = maksimum z duplikatów „Temperature Sensor:CPU”. Wentylatory = RPM.
- Bateria: `root\wmi` `BatteryStatus`, `BatteryFullChargedCapacity`, `BatteryStaticData`. Bypass = `PowerOnline && !Charging && !Discharging`. Cykle = 0 → `None` (ukryte w UI).
- Każdy proces potomny uruchamiany z `CREATE_NO_WINDOW` (0x08000000).
- Poller: `Active` = co 1 s, wszystkie czujniki; `Idle` = co 30 s, bateria + tryby BIOS. Start w `Idle`, bez żadnego okna.
- GPU: NVML wywoływane tylko, gdy urządzenie NVIDIA jest w D0. `nvml.dll` ładowane dynamicznie.
- Dane: `%APPDATA%\MyPrecision\config.json`, `%APPDATA%\MyPrecision\battery-health.json` (maks. 1 wpis/dzień).
- Graphite: `#202020`/`#2d2d2d` powierzchnie, `#3a3a3a` obramowania, `#f3f3f3`/`#9d9d9d` tekst, akcent `#4cc2ff`, tekst na akcencie `#003049`; promienie 8/12/14 px; Segoe UI Variable; `tabular-nums`; segment 180 ms ease-out; flyout 150 ms (8 px + fade); `prefers-reduced-motion` wyłącza animacje.
- Kolor ikony tray: Home = `#4cc2ff`, Campus = `#f3f3f3`, Storage = `#9d9d9d`, Custom = `#f3f3f3`.
- Budżety: w tle < 15 MB RAM, flyout < 300 ms, temperatury ±2 °C względem HWiNFO.
- Commity kończą się linią `Co-Authored-By: Claude <noreply@anthropic.com>`.

## Review Focus

1. **Autostart na baterii** — zadanie tworzone przez `schtasks /SC ONLOGON` domyślnie NIE startuje na baterii (na uczelni). Zadanie musi być tworzone z XML z `DisallowStartIfOnBatteries=false`, `StopIfGoingOnBatteries=false`, `ExecutionTimeLimit=PT0S` (test w Task 11).
2. **Klik w ikonę przy otwartym flyoucie** — blur zamyka flyout tuż przed zdarzeniem kliknięcia, więc bez ochrony flyout natychmiast otwiera się ponownie. Oczekiwane: drugi klik zamyka (toggle). Test logiki debounce w Task 10.
3. **Uszkodzony / ręcznie edytowany `config.json`** (zły JSON albo Custom poza zakresem) — aplikacja musi wystartować z domyślnymi wartościami, zachowując zepsuty plik jako `config.json.bak` (test w Task 3).
4. **Zmiana trybu w trakcie pracy pollera i podwójny klik** — dwa szybkie przełączenia nie mogą uruchomić równolegle dwóch `cctk`; zapisy są serializowane mutexem, a UI blokuje kontrolkę do czasu odpowiedzi (test serializacji w Task 2, blokada w Task 12).
5. **Bateria bez danych / odłączona albo WMI zwraca zera** (`FullChargedCapacity=0`) — brak dzielenia przez zero; procent/zużycie = `None`, UI pokazuje „—” (test w Task 4).

---

## Struktura plików

```
Cargo.toml                         workspace: members = ["crates/core", "src-tauri"]
crates/core/src/
  lib.rs
  dell.rs        ChargeCfg, ThermalMode, parse/format/validate, CctkRunner, Cctk
  profile.rs     BatteryProfile, Profiles, ActiveProfile, detect
  config.rs      Config, load/save
  sensors.rs     typy snapshotów + Telemetry
  battery.rs     BatteryRaw -> BatterySnapshot
  dcim.rs        wiersze DCIM -> DcimReadings
  cpu_load.rs    CpuTimes, load_between
  history.rs     History (bufor kołowy), HistorySample, HealthLog
  tray_icon.rs   render_battery_icon
  autostart.rs   task_xml
src-tauri/
  build.rs, app.manifest, tauri.conf.json, capabilities/default.json
  src/main.rs, lib.rs, state.rs, commands.rs, poller.rs, tray.rs, windows.rs, autostart.rs, probe.rs
  src/platform/{mod,cctk_exec,wmi_battery,wmi_dcim,cpu_times,mem,gpu_power,nvml,system}.rs
src/
  main.ts, App.svelte
  lib/{types,api,labels,format,telemetry.svelte,chart}.ts
  styles/tokens.css
  components/{SegmentedControl,AnimatedNumber,LineChart,Card,Banner,Toasts}.svelte
  views/{Flyout,FullWindow}.svelte
  tests/*.test.ts
```

Logika czysta trafia do `crates/core`, więc `cargo test -p myprecision-core` nie wymaga podniesionych uprawnień (binarka testowa nie dostaje manifestu `requireAdministrator`).

---

### Task 1: Szkielet projektu (workspace, Tauri, Svelte, manifest admina, jedna instancja)

**Files:**
- Create: `Cargo.toml`, `crates/core/Cargo.toml`, `crates/core/src/lib.rs`, `package.json`, `vite.config.ts`, `tsconfig.json`, `svelte.config.js`, `index.html`, `src/main.ts`, `src/App.svelte`, `src-tauri/**` (z `tauri init`), `src-tauri/app.manifest`
- Modify: `.gitignore` (dodaj `target/`)

**Interfaces:**
- Produces: crate `myprecision-core` (lib), crate `myprecision` (bin, zależy od core), skrypty npm: `dev`, `build`, `check` (`svelte-check`), `test` (`vitest run`), `tauri`.

- [ ] **Step 1: Szkielet frontendu** — w katalogu tymczasowym `npm create vite@latest web -- --template svelte-ts`, przenieś pliki do katalogu głównego (bez nadpisywania `docs/`, `.gitignore`), `npm i`, `npm i -D @tauri-apps/cli@^2 vitest @testing-library/svelte jsdom`, `npm i @tauri-apps/api@^2`. Usuń przykładowy licznik i assety szablonu.
- [ ] **Step 2: Tauri** — `npx tauri init --ci -A MyPrecision -W MyPrecision -D ../dist -P http://localhost:5173 --before-dev-command "npm run dev" --before-build-command "npm run build"`. W `tauri.conf.json`: `identifier = "com.myprecision.app"`, `app.windows = []` (start bez okna), `bundle.targets = ["nsis"]`. Dodaj `tauri` z feature `tray-icon`, `image-png` oraz `tauri-plugin-single-instance`.
- [ ] **Step 3: Workspace** — główny `Cargo.toml` z `[workspace] members = ["crates/core","src-tauri"]`, `resolver = "2"`; `[profile.release]` `lto = true`, `codegen-units = 1`, `opt-level = "s"`, `strip = true`, `panic = "abort"`. Core: zależności `serde` (derive), `serde_json`, `thiserror`, `chrono` (serde).
- [ ] **Step 4: Manifest** — `src-tauri/app.manifest` z `requestedExecutionLevel level="requireAdministrator"`, zależnością Microsoft.Windows.Common-Controls 6.0.0.0 i `dpiAwareness PerMonitorV2`; w `build.rs`: `tauri_build::try_build(Attributes::new().windows_attributes(WindowsAttributes::new().app_manifest(include_str!("app.manifest"))))`.
- [ ] **Step 5: Jedna instancja** — w `lib.rs::run()` rejestruj `tauri_plugin_single_instance::init` jako pierwszy plugin (callback na razie pusty; Task 10 otwiera tam pełne okno). Aplikacja nie kończy się przy braku okien: `RunEvent::ExitRequested { code: None, .. }` → `api.prevent_exit()`.
- [ ] **Step 6: Weryfikacja** — `cargo test -p myprecision-core` → PASS (0 testów); `npm run check` → 0 błędów; `cargo build -p myprecision` → sukces.
- [ ] **Step 7: Commit** — `chore: scaffold Tauri 2 + Svelte 5 workspace with admin manifest`

---

### Task 2: `core::dell` — typy BIOS, parsowanie, walidacja, wywołania cctk

**Files:**
- Create: `crates/core/src/dell.rs`
- Test: w tym samym pliku (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(tag = "kind")]
  pub enum ChargeCfg { Standard, Adaptive, PrimAcUse, Express, Custom { start: u8, stop: u8 } }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum ThermalMode { Optimized, Cool, Quiet, UltraPerformance }
  pub enum DellError { NotInstalled, InvalidRange { start: u8, stop: u8 }, Unparsable(String), Failed { code: i32, message: String } } // thiserror, Serialize jako string
  pub fn validate_custom(start: u8, stop: u8) -> Result<(), DellError>;
  pub fn parse_charge_cfg(output: &str) -> Result<ChargeCfg, DellError>;
  pub fn format_charge_cfg(cfg: ChargeCfg) -> String;   // "Standard" | "Custom:75-80"
  pub fn parse_thermal(output: &str) -> Result<ThermalMode, DellError>;
  pub trait CctkRunner: Send + Sync { fn run(&self, args: &[&str]) -> Result<(i32, String), DellError>; }
  pub struct Cctk<R: CctkRunner> { /* runner + Mutex<()> */ }
  impl<R: CctkRunner> Cctk<R> {
      pub fn new(runner: R) -> Self;
      pub fn get_charge_cfg(&self) -> Result<ChargeCfg, DellError>;
      pub fn set_charge_cfg(&self, cfg: ChargeCfg) -> Result<ChargeCfg, DellError>; // zapis + ponowny odczyt
      pub fn get_thermal(&self) -> Result<ThermalMode, DellError>;
      pub fn set_thermal(&self, mode: ThermalMode) -> Result<ThermalMode, DellError>;
  }
  ```

- [ ] **Step 1: Testy (fałszywy runner z kolejką odpowiedzi i zapisem argumentów)**
  ```rust
  #[test] fn parses_standard() { assert_eq!(parse_charge_cfg("PrimaryBattChargeCfg=Standard\r\n").unwrap(), ChargeCfg::Standard); }
  #[test] fn parses_custom() { assert_eq!(parse_charge_cfg("PrimaryBattChargeCfg=Custom:75-80").unwrap(), ChargeCfg::Custom{start:75,stop:80}); }
  #[test] fn parses_all_simple_variants() { /* Adaptive, PrimAcUse, Express */ }
  #[test] fn parses_thermal() { assert_eq!(parse_thermal("ThermalManagement=UltraPerformance").unwrap(), ThermalMode::UltraPerformance); }
  #[test] fn garbage_is_unparsable() { assert!(matches!(parse_charge_cfg("Error: something"), Err(DellError::Unparsable(_)))); }
  #[test] fn formats_custom() { assert_eq!(format_charge_cfg(ChargeCfg::Custom{start:50,stop:60}), "Custom:50-60"); }
  #[test] fn validates_ranges() {
      assert!(validate_custom(50,55).is_ok()); assert!(validate_custom(95,100).is_ok());
      assert!(validate_custom(49,60).is_err()); assert!(validate_custom(90,94).is_err());
      assert!(validate_custom(75,79).is_err()); assert!(validate_custom(96,100).is_err());
  }
  #[test] fn set_charge_passes_exact_arg_and_rereads() {
      // runner: [(0,""), (0,"PrimaryBattChargeCfg=Custom:75-80")]
      // expect args[0] == ["--PrimaryBattChargeCfg=Custom:75-80"], args[1] == ["--PrimaryBattChargeCfg"], result Custom{75,80}
  }
  #[test] fn set_invalid_custom_never_calls_runner() { /* Custom{75,78} → InvalidRange, runner.calls == 0 */ }
  #[test] fn nonzero_exit_is_failed_with_output() { /* (58,"Password is required") → Failed{code:58, message:"Password is required"} */ }
  #[test] fn concurrent_sets_are_serialized() {
      // runner śpi 50 ms i rejestruje max równoległych wywołań; 2 wątki set_thermal → max_concurrent == 1
  }
  ```
- [ ] **Step 2:** `cargo test -p myprecision-core dell` → FAIL (brak modułu).
- [ ] **Step 3: Implementacja** — parsowanie: pierwsza linia zawierająca `=` z kluczem `PrimaryBattChargeCfg`/`ThermalManagement`, wartość po `=` przycięta. Wszystkie metody `Cctk` biorą ten sam `Mutex<()>` na czas wywołania (zapis + odczyt jako jedna sekcja).
- [ ] **Step 4:** `cargo test -p myprecision-core dell` → PASS.
- [ ] **Step 5: Commit** — `feat(core): cctk types, parsing and serialized runner`

---

### Task 3: `core::profile` + `core::config`

**Files:**
- Create: `crates/core/src/profile.rs`, `crates/core/src/config.rs`

**Interfaces:**
- Consumes: `ChargeCfg`, `validate_custom` (Task 2).
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")]
  pub enum BatteryProfile { Home, Campus, Storage }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
  pub struct Profiles { pub home: ChargeCfg, pub campus: ChargeCfg, pub storage: ChargeCfg }
  impl Default for Profiles; impl Profiles { pub fn get(&self, p: BatteryProfile) -> ChargeCfg; }
  #[derive(Clone, Debug, PartialEq, Serialize)] #[serde(tag = "kind", rename_all = "camelCase")]
  pub enum ActiveProfile { Known { profile: BatteryProfile }, Other { cfg: ChargeCfg } }
  pub fn detect(cfg: ChargeCfg, profiles: &Profiles) -> ActiveProfile;
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] #[serde(rename_all = "camelCase", default)]
  pub struct Config { pub version: u32, pub profiles: Profiles, pub optimizer_warning_dismissed: bool }
  pub fn load(path: &Path) -> Config;          // nigdy nie zwraca błędu
  pub fn save(path: &Path, cfg: &Config) -> std::io::Result<()>;  // zapis atomowy: .tmp + rename
  ```

- [ ] **Step 1: Testy**
  ```rust
  #[test] fn defaults_match_spec() { let p = Profiles::default();
      assert_eq!(p.home, ChargeCfg::Custom{start:75,stop:80}); assert_eq!(p.campus, ChargeCfg::Standard);
      assert_eq!(p.storage, ChargeCfg::Custom{start:50,stop:60}); }
  #[test] fn detects_known() { assert_eq!(detect(ChargeCfg::Standard, &Profiles::default()), ActiveProfile::Known{profile:BatteryProfile::Campus}); }
  #[test] fn detects_other() { assert_eq!(detect(ChargeCfg::Custom{start:60,stop:90}, &Profiles::default()), ActiveProfile::Other{cfg:ChargeCfg::Custom{start:60,stop:90}}); }
  #[test] fn load_missing_file_gives_default() { /* tempdir, brak pliku → Config::default() */ }
  #[test] fn roundtrip() { /* save → load == oryginał */ }
  #[test] fn corrupt_json_gives_default_and_backup() { /* zapisz "{nope" → load == default; istnieje config.json.bak z "{nope" */ }
  #[test] fn invalid_custom_in_file_falls_back_per_profile() { /* home = Custom{75,77} → home == default, storage zachowany */ }
  ```
- [ ] **Step 2:** `cargo test -p myprecision-core profile config` → FAIL.
- [ ] **Step 3: Implementacja** — w `load` po deserializacji każdy profil `Custom` nieprzechodzący `validate_custom` zastąp wartością z `Profiles::default()`. Do testów dodaj dev-dependency `tempfile`.
- [ ] **Step 4:** testy → PASS.
- [ ] **Step 5: Commit** — `feat(core): battery profiles and resilient config`

---

### Task 4: `core::sensors` — typy, bateria, DCIM, obciążenie CPU

**Files:**
- Create: `crates/core/src/sensors.rs`, `crates/core/src/battery.rs`, `crates/core/src/dcim.rs`, `crates/core/src/cpu_load.rs`

**Interfaces:**
- Produces (wszystkie struktury `Serialize`, `#[serde(rename_all = "camelCase")]`, `Clone, Debug, PartialEq`):
  ```rust
  pub enum PowerState { Charging, Discharging, Bypass }
  pub struct BatteryRaw { pub power_online: bool, pub charging: bool, pub discharging: bool,
      pub charge_rate_mw: u32, pub discharge_rate_mw: u32, pub remaining_mwh: u32, pub voltage_mv: u32,
      pub full_mwh: u32, pub design_mwh: u32, pub cycles: u32 }
  pub struct BatterySnapshot { pub percent: Option<f32>, pub state: PowerState, pub power_w: f32 /* + ładowanie, − rozładowanie */,
      pub remaining_mwh: u32, pub full_mwh: Option<u32>, pub design_mwh: Option<u32>, pub wear_pct: Option<f32>,
      pub cycles: Option<u32>, pub voltage_v: f32 }
  pub fn battery_from_raw(raw: &BatteryRaw) -> BatterySnapshot;
  pub struct DcimRow { pub element_name: String, pub current_reading: i64 }
  pub struct DcimReadings { pub cpu_c: Option<f32>, pub dimm_c: Option<f32>, pub skin_c: Option<f32>, pub fans: Vec<FanReading> }
  pub struct FanReading { pub name: String /* "CPU" | "GPU" | oryginalna nazwa */, pub rpm: u32 }
  pub fn parse_dcim(rows: &[DcimRow]) -> DcimReadings;
  pub struct CpuTimes { pub idle: u64, pub kernel: u64, pub user: u64 }  // kernel zawiera idle
  pub fn load_between(prev: CpuTimes, now: CpuTimes) -> Option<f32>;   // 0..=100
  pub struct CpuSnapshot { pub load_pct: Option<f32>, pub temp_c: Option<f32> }
  pub struct MemSnapshot { pub used_mb: u32, pub total_mb: u32 }
  #[serde(tag = "state")] pub enum GpuSnapshot { Asleep, Unavailable, Active { temp_c: u32, load_pct: u32 } }
  pub struct Telemetry { pub ts_ms: i64, pub battery: Option<BatterySnapshot>, pub cpu: CpuSnapshot, pub gpu: GpuSnapshot,
      pub fans: Vec<FanReading>, pub mem: Option<MemSnapshot>, pub dimm_c: Option<f32>, pub skin_c: Option<f32> }
  ```

- [ ] **Step 1: Testy**
  ```rust
  #[test] fn wear_from_real_battery() { // design 67914, full 58709
      let s = battery_from_raw(&raw(full=58709, design=67914, ..)); assert_eq!(s.wear_pct, Some(13.6)); }
  #[test] fn bypass_when_online_idle() { /* online, !charging, !discharging → Bypass, power_w == 0.0 */ }
  #[test] fn charging_power_positive() { /* charging, charge_rate 45000 → Charging, power_w == 45.0 */ }
  #[test] fn discharging_power_negative() { /* discharge_rate 12500 → power_w == -12.5 */ }
  #[test] fn offline_without_flags_is_discharging() { /* !online, !charging, !discharging → Discharging */ }
  #[test] fn percent_clamped() { /* remaining 59000, full 58709 → Some(100.0) */ }
  #[test] fn zero_capacities_give_none() { /* full 0, design 0 → percent None, wear None, full_mwh None */ }
  #[test] fn zero_cycles_hidden() { /* cycles 0 → None; 12 → Some(12) */ }
  #[test] fn dcim_cpu_takes_max_of_duplicates() { /* CPU 79 i 58 → cpu_c Some(79.0), bez skalowania */ }
  #[test] fn dcim_fans_named() { /* "Fan Speed Sensor:Processor Fan" 2400 → FanReading{name:"CPU",rpm:2400}; "Video Fan" → "GPU" */ }
  #[test] fn dcim_empty_gives_none() { /* [] → wszystko None, fans pusty */ }
  #[test] fn cpu_load_basic() { /* prev{0,0,0} now{idle 750, kernel 900, user 100} → Some(25.0) */ }
  #[test] fn cpu_load_zero_delta_none() { /* te same próbki → None */ }
  ```
- [ ] **Step 2:** `cargo test -p myprecision-core` → FAIL.
- [ ] **Step 3: Implementacja** — zużycie `(1 - full/design)*100` zaokrąglone do 0,1; procent `remaining/full*100` obcięty do 0..=100 i zaokrąglony do 0,1; wartości DCIM ≤ 0 pomijaj.
- [ ] **Step 4:** testy → PASS.
- [ ] **Step 5: Commit** — `feat(core): sensor types, battery math, DCIM parsing, CPU load`

---

### Task 5: `core::history` — bufor 30 min i dziennik zdrowia baterii

**Files:**
- Create: `crates/core/src/history.rs`

**Interfaces:**
- Consumes: `Telemetry` (Task 4).
- Produces:
  ```rust
  #[derive(Serialize, Clone, Debug, PartialEq)] #[serde(rename_all = "camelCase")]
  pub struct HistorySample { pub ts_ms: i64, pub cpu_temp: Option<f32>, pub cpu_load: Option<f32>, pub gpu_temp: Option<u32>,
      pub battery_pct: Option<f32>, pub battery_w: Option<f32> }
  impl From<&Telemetry> for HistorySample;
  pub struct History { /* VecDeque, cap */ }
  impl History { pub const CAPACITY: usize = 1800; pub fn new() -> Self; pub fn push(&mut self, s: HistorySample);
      pub fn range(&self, minutes: u32, now_ms: i64) -> Vec<HistorySample>; }
  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] #[serde(rename_all = "camelCase")]
  pub struct HealthEntry { pub date: chrono::NaiveDate, pub full_mwh: u32, pub design_mwh: u32, pub cycles: Option<u32> }
  pub struct HealthLog { path: PathBuf, entries: Vec<HealthEntry> }
  impl HealthLog { pub fn open(path: &Path) -> Self; pub fn entries(&self) -> &[HealthEntry];
      pub fn record(&mut self, entry: HealthEntry) -> std::io::Result<bool>; } // false gdy dzień już zapisany
  ```

- [ ] **Step 1: Testy**
  ```rust
  #[test] fn ring_drops_oldest() { /* push 1801 → len 1800, pierwszy ts == 1 */ }
  #[test] fn range_filters_by_time() { /* próbki co 1 s przez 20 min; range(5, now) → 300 lub 301 próbek, wszystkie ts >= now-300_000 */ }
  #[test] fn health_one_per_day() { /* record(dzień X) → true; record(dzień X, inne full) → false, wpis bez zmian; record(X+1) → true */ }
  #[test] fn health_persists() { /* record, ponowne open → ten sam wpis */ }
  #[test] fn health_corrupt_file_starts_empty() { /* "xx" → entries() pusty */ }
  ```
- [ ] **Step 2:** FAIL → **Step 3:** implementacja (zapis atomowy jak w config) → **Step 4:** PASS.
- [ ] **Step 5: Commit** — `feat(core): telemetry history ring and daily battery health log`

---

### Task 6: Odczyty platformy (cctk, WMI bateria, DCM, CPU, RAM, system) + tryb `--probe`

**Files:**
- Create: `src-tauri/src/platform/{mod,cctk_exec,wmi_battery,wmi_dcim,cpu_times,mem,system}.rs`, `src-tauri/src/probe.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` (dodaj `wmi`, `windows`, `serde_json`)

**Interfaces:**
- Consumes: `CctkRunner`, `DellError` (Task 2), `BatteryRaw`, `DcimRow`, `CpuTimes`, `MemSnapshot` (Task 4).
- Produces:
  ```rust
  pub struct ExeCctkRunner { path: PathBuf }
  impl ExeCctkRunner { pub const DEFAULT_PATH: &str = r"C:\Program Files (x86)\Dell\Command Configure\X86_64\cctk.exe";
      pub fn locate() -> Option<Self>; }            // None gdy plik nie istnieje
  impl CctkRunner for ExeCctkRunner;               // stdout+stderr, CREATE_NO_WINDOW
  pub struct WmiReaders { /* WMIConnection root\wmi, Option<WMIConnection> root\dcim\sysman */ } // !Send — żyje w wątku pollera
  impl WmiReaders { pub fn new() -> anyhow::Result<Self>; pub fn battery(&self) -> Option<BatteryRaw>;
      pub fn dcim(&self) -> Option<Vec<DcimRow>>; pub fn dcm_available(&self) -> bool; }
  pub fn cpu_times() -> CpuTimes;                  // GetSystemTimes, FILETIME → u64
  pub fn mem() -> Option<MemSnapshot>;             // GlobalMemoryStatusEx
  pub fn is_elevated() -> bool;                    // IsUserAnAdmin
  pub fn optimizer_running() -> bool;              // usługa "DellOptimizer" lub "DellTechHub" w stanie Running (sc query, CREATE_NO_WINDOW)
  pub fn run_probe(out: &Path) -> anyhow::Result<()>; // `myprecision.exe --probe <plik>`: JSON {chargeCfg, thermal, battery, dcimRows, dcim, cpuLoad, mem, elevated, optimizerRunning}
  ```

- [ ] **Step 1: Implementacja** — zapytania WMI: `SELECT * FROM BatteryStatus`, `BatteryFullChargedCapacity`, `BatteryStaticData` (`DesignedCapacity`), `BatteryCycleCount`; `SELECT ElementName, CurrentReading FROM DCIM_NumericSensor`. Brak namespace DCM → `dcim()` = `None`, bez paniki. W `main`: jeśli `args[1] == "--probe"` → `run_probe(args[2])` i wyjście przed startem Tauri. Dodaj `anyhow`.
- [ ] **Step 2: Kompilacja** — `cargo build -p myprecision` → sukces, `cargo clippy -p myprecision -- -D warnings` → 0 ostrzeżeń.
- [ ] **Step 3: Weryfikacja na urządzeniu** — binarka wymaga admina, więc uruchom ją z podniesieniem i zapisem do pliku:
  `powershell -NoProfile -Command "Start-Process -Verb RunAs -Wait -FilePath '<repo>\target\debug\myprecision.exe' -ArgumentList '--probe','<repo>\.probe\probe.json'"` (użytkownik zatwierdza UAC).
  Oczekiwane w `.probe/probe.json`: `chargeCfg` = `{"kind":"Standard"}` lub aktualny profil, `thermal` = `"Optimized"`, `battery.designMwh` = 67914, `dcim.cpuC` ≠ null, dwa wentylatory, `elevated` = true.
- [ ] **Step 4: Commit** — `feat: Windows sensor readers, cctk runner and --probe mode`

---

### Task 7: GPU — stan zasilania i NVML

**Files:**
- Create: `src-tauri/src/platform/gpu_power.rs`, `src-tauri/src/platform/nvml.rs`
- Modify: `src-tauri/src/probe.rs` (dodaj `gpu`, `gpuPowerState`)

**Interfaces:**
- Consumes: `GpuSnapshot` (Task 4).
- Produces:
  ```rust
  pub enum DevicePower { D0, Sleeping, NotFound }
  pub fn nvidia_power_state() -> DevicePower;   // SetupAPI, klasa DISPLAY, HardwareID zawiera "VEN_10DE", DEVPKEY_Device_PowerData → CM_POWER_DATA.PD_MostRecentPowerState
  pub struct Nvml { /* libloading::Library + wskaźniki funkcji + handle urządzenia 0 */ }
  impl Nvml { pub fn load() -> Option<Self>;     // nvml.dll; nvmlInit_v2, nvmlDeviceGetHandleByIndex_v2, nvmlDeviceGetTemperature (NVML_TEMPERATURE_GPU=0), nvmlDeviceGetUtilizationRates
      pub fn read(&self) -> Option<(u32, u32)>; }
  pub struct GpuReader { nvml: Option<Nvml>, tried: bool }
  impl GpuReader { pub fn new() -> Self; pub fn read(&mut self) -> GpuSnapshot; }
  ```

- [ ] **Step 1: Implementacja** — `GpuReader::read`: `NotFound` → `Unavailable`; `Sleeping` → `Asleep` (bez NVML); `D0` → przy pierwszym D0 leniwie `Nvml::load()` (nigdy wcześniej — `nvmlInit` mógłby wybudzić kartę); brak biblioteki/błąd → `Unavailable`. `nvmlShutdown` w `Drop`.
- [ ] **Step 2:** `cargo clippy -p myprecision -- -D warnings` → 0 ostrzeżeń.
- [ ] **Step 3: Weryfikacja** — probe przy bezczynności: `gpuPowerState` = Sleeping, `gpu` = `{"state":"Asleep"}`. Probe podczas obciążenia GPU (np. film w przeglądarce z akceleracją na dGPU lub `nvidia-smi`): `{"state":"Active",…}` z temperaturą 30–90.
- [ ] **Step 4: Commit** — `feat: GPU power-state gate and dynamic NVML reader`

---

### Task 8: Stan aplikacji, poller i komendy Tauri

**Files:**
- Create: `src-tauri/src/state.rs`, `src-tauri/src/poller.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: wszystko z Tasks 2–7.
- Produces:
  ```rust
  #[derive(Serialize, Clone)] #[serde(rename_all = "camelCase")]
  pub struct Availability { pub cctk: bool, pub dcm: bool, pub admin: bool, pub optimizer_running: bool }
  #[derive(Serialize, Clone)] #[serde(rename_all = "camelCase")]
  pub struct AppState { pub charge: Option<ChargeCfg>, pub active_profile: Option<ActiveProfile>, pub thermal: Option<ThermalMode>,
      pub profiles: Profiles, pub battery: Option<BatterySnapshot>, pub availability: Availability,
      pub autostart: bool, pub optimizer_warning_dismissed: bool }
  pub enum PollMode { Active, Idle }
  pub struct Core { pub cctk: Option<Cctk<ExeCctkRunner>>, pub config: Mutex<Config>, pub history: Mutex<History>,
      pub health: Mutex<HealthLog>, pub snapshot: Mutex<AppState>, poller_tx: Sender<PollMode> }
  impl Core { pub fn set_poll_mode(&self, m: PollMode); pub fn refresh_bios(&self) -> AppState; }  // ponowny odczyt cctk → snapshot
  // zdarzenia: "telemetry" (Telemetry), "state-changed" (AppState)
  // komendy (#[tauri::command], błędy jako String po angielsku):
  get_state() -> AppState
  set_battery_profile(profile: BatteryProfile) -> Result<AppState, String>
  set_thermal_mode(mode: ThermalMode) -> Result<AppState, String>
  get_history(minutes: u32) -> Vec<HistorySample>
  get_health_log() -> Vec<HealthEntry>
  set_autostart(enabled: bool) -> Result<bool, String>          // implementacja w Task 11; tu zwraca Err("Not available yet")
  dismiss_optimizer_warning() -> ()
  open_full_window() -> ()                                       // implementacja w Task 10
  ```

- [ ] **Step 1: Poller** — osobny wątek posiadający `WmiReaders`, `GpuReader`, poprzednie `CpuTimes`. Pętla `rx.recv_timeout(interval)`; zmiana trybu działa natychmiast (nowy tryb → od razu jedna pełna próbka). `Active`: pełna `Telemetry` → `history.push`, `emit("telemetry")`. `Idle`: bateria + `refresh_bios()`; jeśli `active_profile` lub `thermal` się zmienił → `emit("state-changed")` i odświeżenie tray (callback przekazany z Task 10; na razie no-op). Po każdym odczycie baterii `health.record(HealthEntry{ date: Local::now().date_naive(), .. })` gdy `full_mwh` i `design_mwh` są `Some`.
- [ ] **Step 2: Komunikaty błędów** — mapowanie `DellError` → tekst: `NotInstalled` → „Dell Command | Configure (cctk.exe) not found.”; `InvalidRange{s,t}` → „Invalid charge range: {s}–{t}%.”; `Failed{code,message}` → „BIOS rejected the change (code {code}): {message}”; `Unparsable` → „Could not read the BIOS setting.” Po błędzie zapisu i tak wykonaj `refresh_bios()` i wyemituj `state-changed`.
- [ ] **Step 3:** Rejestracja komend w `invoke_handler`, `Core` w `app.manage`, poller start w `setup` w trybie `Idle`. `capabilities/default.json`: okna `flyout`, `main`; uprawnienia `core:default`, `core:window:allow-close`, `core:event:default`.
- [ ] **Step 4: Weryfikacja** — `cargo clippy -p myprecision -- -D warnings` → 0; `cargo test -p myprecision-core` → PASS. `open_full_window` na tym etapie zwraca `()` bez działania (Task 10).
- [ ] **Step 5: Commit** — `feat: app core state, Active/Idle poller and Tauri commands`

---

### Task 9: Fundament UI — tokeny, API, SegmentedControl, AnimatedNumber, wykres

**Files:**
- Create: `src/styles/tokens.css`, `src/lib/{types,api,labels,format,chart}.ts`, `src/lib/telemetry.svelte.ts`, `src/components/{SegmentedControl,AnimatedNumber,LineChart,Card}.svelte`, `src/tests/{SegmentedControl,labels,chart}.test.ts`
- Modify: `vite.config.ts` (vitest: `environment: 'jsdom'`, plugin svelte z `resolve.conditions: ['browser']` w testach), `src/App.svelte`, `src/main.ts`

**Interfaces:**
- Consumes: kształty JSON z Task 8 (`AppState`, `Telemetry`, `HistorySample`, `HealthEntry`).
- Produces:
  ```ts
  // types.ts — lustro typów Rust (camelCase, ChargeCfg = {kind:'Standard'} | … | {kind:'Custom', start:number, stop:number})
  // api.ts
  export const api: { getState(): Promise<AppState>; setBatteryProfile(p: BatteryProfile): Promise<AppState>;
    setThermalMode(m: ThermalMode): Promise<AppState>; getHistory(min: number): Promise<HistorySample[]>;
    getHealthLog(): Promise<HealthEntry[]>; setAutostart(on: boolean): Promise<boolean>;
    dismissOptimizerWarning(): Promise<void>; openFullWindow(): Promise<void> };
  // labels.ts
  export const PROFILE_LABEL: Record<BatteryProfile, string>;   // Home / Campus / Storage
  export const THERMAL_LABEL: Record<ThermalMode, string>;
  export function activeProfileLabel(a: ActiveProfile | null): string;  // Other Custom → "Custom (60–90%)"; Other Adaptive → "Adaptive"; Express → "Express"; PrimAcUse → "Primarily AC"; null → "—"
  export function chargeCfgLabel(c: ChargeCfg): string;          // "75–80%" | "Standard (up to 100%)"
  // chart.ts
  export function toPolyline(pts: {t:number; v:number|null}[], w: number, h: number, tMin: number, tMax: number, vMin: number, vMax: number): string[]; // osobne odcinki przy null
  // telemetry.svelte.ts — rune store: latest Telemetry, AppState; subskrypcja "telemetry" i "state-changed"
  // SegmentedControl props: { options: {value: string; label: string}[]; value: string | null; disabled?: boolean; busy?: boolean; onchange(v: string): void }
  // AnimatedNumber props: { value: number | null; decimals?: number; suffix?: string }  // null → "—"
  // LineChart props: { samples: {t:number; v:number|null}[]; minutes: number; unit: string; min?: number; max?: number }  // canvas, DPR-aware
  ```

- [ ] **Step 1: Testy**
  ```ts
  test('renders options as radios with aria-checked', ...)       // value 'home' → tylko „Home” ma aria-checked="true"
  test('click calls onchange with value', ...)
  test('null value → no option checked, indicator hidden', ...)  // profil „Custom”
  test('disabled/busy → click does not call onchange', ...)
  test('ArrowRight moves to next option and calls onchange', ...)
  test('activeProfileLabel Other Custom', () => expect(activeProfileLabel({kind:'other', cfg:{kind:'Custom',start:60,stop:90}})).toBe('Custom (60–90%)'))
  test('toPolyline splits on null', ...)                          // [1,null,3,4] → 2 odcinki
  ```
- [ ] **Step 2:** `npm test` → FAIL.
- [ ] **Step 3: Implementacja** — `tokens.css`: zmienne z Global Constraints (`--bg`, `--surface`, `--border`, `--text`, `--text-2`, `--accent`, `--on-accent`, `--r-ctl: 8px`, `--r-card: 12px`, `--r-flyout: 14px`, `--ease: cubic-bezier(.2,.8,.2,1)`), `body { background: transparent }` (Mica/Acrylic), blok `@media (prefers-reduced-motion: reduce)` zerujący `transition`/`animation`. SegmentedControl: wskaźnik jako jeden absolutnie pozycjonowany element przesuwany `transform` (180 ms), `role="radiogroup"`/`role="radio"`. AnimatedNumber: interpolacja `requestAnimationFrame` 300 ms, `tabular-nums`. `main.ts`: montuje `Flyout` lub `FullWindow` wg `getCurrentWindow().label` (widoki-zaślepki do Task 12/13).
- [ ] **Step 4:** `npm test` → PASS; `npm run check` → 0 błędów.
- [ ] **Step 5: Commit** — `feat(ui): Graphite tokens, typed API, segmented control, animated numbers, chart`

---

### Task 10: Tray i cykl życia okien

**Files:**
- Create: `crates/core/src/tray_icon.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/windows.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs` (`open_full_window`), `src-tauri/src/poller.rs` (callback tray)

**Interfaces:**
- Consumes: `Core`, `AppState`, `PollMode` (Task 8).
- Produces:
  ```rust
  // core
  pub fn render_battery_icon(color: [u8; 3], fill_pct: Option<u8>, size: u32) -> Vec<u8>;  // RGBA size×size, obrys baterii + wypełnienie proporcjonalne
  pub fn icon_color(active: Option<&ActiveProfile>) -> [u8; 3];                         // wg Global Constraints
  pub struct ToggleGuard { last_hide_ms: Option<i64> }
  impl ToggleGuard { pub const WINDOW_MS: i64 = 300; pub fn on_hidden(&mut self, now_ms: i64); pub fn should_open(&mut self, now_ms: i64) -> bool; }
  // app
  pub fn tray::build(app: &AppHandle) -> tauri::Result<()>;
  pub fn tray::refresh(app: &AppHandle, state: &AppState, cpu_c: Option<f32>);   // ikona, tooltip, zaznaczenia w menu
  pub fn windows::toggle_flyout(app: &AppHandle);
  pub fn windows::open_full(app: &AppHandle);
  ```

- [ ] **Step 1: Testy (core)**
  ```rust
  #[test] fn icon_has_expected_size() { assert_eq!(render_battery_icon([76,194,255], Some(50), 32).len(), 32*32*4); }
  #[test] fn icon_fill_grows_with_percent() { /* liczba nieprzezroczystych pikseli: 80% > 20% */ }
  #[test] fn colors_per_profile() { /* Home → [0x4c,0xc2,0xff]; Campus → [0xf3,0xf3,0xf3]; Storage → [0x9d,0x9d,0x9d]; Other → [0xf3,0xf3,0xf3] */ }
  #[test] fn click_right_after_blur_hide_does_not_reopen() { /* on_hidden(1000); should_open(1150) == false; should_open(1400) == true */ }
  ```
- [ ] **Step 2:** FAIL → **Step 3: Implementacja**
  - Tray: lewy klik (`TrayIconEvent::Click`, `MouseButton::Left`, `MouseButtonState::Up`) → `toggle_flyout`. Menu natywne: radio-checkboxy profili („Home 75–80%”, „Campus 100%”, „Storage 50–60%”), podmenu „Thermal mode” z 4 trybami, „Open MyPrecision”, „Start at sign-in” (check), „Quit”. Akcje menu wołają te same funkcje co komendy, potem `refresh` + `emit("state-changed")`. Tooltip: `MyPrecision — 82% · Home · CPU 54 °C` (pomijaj brakujące części).
  - Flyout: etykieta `flyout`, 360×520 logicznych px, bez dekoracji, `always_on_top`, `skip_taskbar`, `transparent`, efekt `Acrylic`, pozycja: prawy-dolny róg obszaru roboczego monitora z ikoną, margines 12 px. Jeśli istnieje → zamknij (toggle). `WindowEvent::Focused(false)` → `close()` + `ToggleGuard::on_hidden`. `Destroyed` → jeśli nie ma już okien → `PollMode::Idle`. Utworzenie → `PollMode::Active`.
  - Pełne okno: etykieta `main`, 960×640, min 760×520, efekt `Mica`, dekoracje systemowe; jeśli istnieje → `set_focus`. Otwarcie zamyka flyout. Single-instance callback → `open_full`.
- [ ] **Step 4:** `cargo test -p myprecision-core tray_icon` → PASS; `cargo clippy -p myprecision -- -D warnings` → 0.
- [ ] **Step 5: Weryfikacja ręczna** — `npm run tauri dev` w terminalu administratora: ikona widoczna, menu przełącza profil (sprawdź `cctk --PrimaryBattChargeCfg`), kolor ikony się zmienia, lewy klik otwiera/zamyka flyout, klik poza nim zamyka, w Menedżerze zadań znika proces `msedgewebview2` po zamknięciu.
- [ ] **Step 6: Commit** — `feat: tray icon, native menu and flyout/full window lifecycle`

---

### Task 11: Autostart (Harmonogram zadań)

**Files:**
- Create: `crates/core/src/autostart.rs`, `src-tauri/src/autostart.rs`
- Modify: `src-tauri/src/commands.rs` (`set_autostart`), `src-tauri/src/state.rs` (`autostart` w `AppState`), `src-tauri/src/tray.rs` (check w menu)

**Interfaces:**
- Produces:
  ```rust
  pub const TASK_NAME: &str = "MyPrecision";
  pub fn task_xml(exe_path: &str, user_id: &str) -> String;   // core; user_id = "DOMENA\\użytkownik"
  pub fn enable() -> Result<(), String>;    // zapis XML jako UTF-16LE z BOM do %TEMP%, schtasks /Create /TN MyPrecision /XML <plik> /F, usunięcie pliku
  pub fn disable() -> Result<(), String>;   // schtasks /Delete /TN MyPrecision /F
  pub fn is_enabled() -> bool;              // schtasks /Query /TN MyPrecision → kod 0
  ```

- [ ] **Step 1: Testy**
  ```rust
  #[test] fn xml_runs_on_battery() { let x = task_xml(r"C:\Program Files\MyPrecision\myprecision.exe", r"PC\DELL");
      assert!(x.contains("<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>"));
      assert!(x.contains("<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>"));
      assert!(x.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
      assert!(x.contains("<RunLevel>HighestAvailable</RunLevel>"));
      assert!(x.contains("<LogonTrigger>")); assert!(x.contains(r"<UserId>PC\DELL</UserId>")); }
  #[test] fn xml_escapes_path() { /* ścieżka z "&" → "&amp;" */ }
  #[test] fn xml_declares_utf16() { assert!(task_xml("a","b").starts_with(r#"<?xml version="1.0" encoding="UTF-16"?>"#)); }
  ```
- [ ] **Step 2:** FAIL → **Step 3:** implementacja (XML wg schematu Task Scheduler 1.2: `LogonTrigger` z `UserId`, `Principal` z `LogonType=InteractiveToken`, `MultipleInstancesPolicy=IgnoreNew`, `Exec/Command`) → **Step 4:** PASS.
- [ ] **Step 5: Weryfikacja** — włącz z menu, `schtasks /Query /TN MyPrecision /XML` pokazuje oba ustawienia baterii `false`; wyloguj/zaloguj → ikona pojawia się bez monitu UAC.
- [ ] **Step 6: Commit** — `feat: elevated logon autostart that also runs on battery`

---

### Task 12: Widok flyout

**Files:**
- Create: `src/views/Flyout.svelte`, `src/tests/Flyout.test.ts`

**Interfaces:**
- Consumes: `api`, store telemetrii, `SegmentedControl`, `AnimatedNumber`, `labels` (Task 9).

Układ (od góry): nagłówek (bateria % dużą cyfrą, stan: „Charging 45 W” / „Discharging 12.5 W” / „Plugged in — battery bypassed”, aktywny profil), kontrolka „Battery profile” (3 segmenty), kontrolka „Thermal mode” (4 segmenty), siatka 2×2 kafli: CPU (°C + %), GPU (°C + % / „Asleep” / „Unavailable”), wentylatory (RPM obu), RAM (użyte/całk. GB), stopka: przycisk „Open full window”.

- [ ] **Step 1: Testy (z zamockowanym `api` i store)**
  ```ts
  test('Other profile shows "Custom (60–90%)" and no segment checked', ...)
  test('clicking Home calls setBatteryProfile("home") and disables control until resolved', ...)  // drugi klik w trakcie → brak drugiego wywołania
  test('rejected setThermalMode shows error toast text', ...)
  test('gpu Asleep renders "Asleep"', ...)
  ```
- [ ] **Step 2:** FAIL → **Step 3:** implementacja; wejście: `transform: translateY(8px)` + `opacity 0` → 0 / 1 w 150 ms; zaokrąglenie 14 px; pierwszy render z `api.getState()` bez czekania na telemetrię → **Step 4:** PASS, `npm run check` → 0.
- [ ] **Step 5: Commit** — `feat(ui): tray flyout view`

---

### Task 13: Pełne okno

**Files:**
- Create: `src/views/FullWindow.svelte`, `src/views/full/{Overview,Battery,Sensors}.svelte`, `src/tests/FullWindow.test.ts`

**Interfaces:**
- Consumes: jak Task 12 + `LineChart`, `api.getHistory`, `api.getHealthLog`, `api.setAutostart`.

Nawigacja boczna (3 sekcje): **Overview** (te same kontrolki co flyout, większe, opisy profili: Home „Holds 75–80% — ideal when plugged in at a desk”, Campus „Charges to 100% before you head out”, Storage „50–60% for long-term storage”), **Battery** (pojemność fabryczna / obecna, zużycie %, cykle tylko gdy ≠ null, napięcie, moc; wykres dziennej historii `fullMwh` z `getHealthLog`; wykres % i W z ostatnich 30 min), **Sensors** (wykresy 30 min: temp. CPU, obciążenie CPU, temp. GPU; kafle: DIMM, obudowa (SKIN), wentylatory, RAM), stopka sekcji: przełącznik „Start at sign-in”. Historia: `getHistory(30)` przy otwarciu, potem dopisywanie z `telemetry`.

- [ ] **Step 1: Testy**
  ```ts
  test('cycles hidden when null', ...)
  test('wear shows "13.6%" for wearPct 13.6', ...)          // format en-US
  test('autostart toggle calls setAutostart(true)', ...)
  ```
- [ ] **Step 2:** FAIL → **Step 3:** implementacja → **Step 4:** PASS, `npm run check` → 0.
- [ ] **Step 5: Commit** — `feat(ui): full window with battery health and 30-minute charts`

---

### Task 14: Banery i toasty błędów

**Files:**
- Create: `src/components/{Banner,Toasts}.svelte`, `src/lib/banners.ts`, `src/tests/banners.test.ts`
- Modify: `src/views/Flyout.svelte`, `src/views/FullWindow.svelte`

**Interfaces:**
- Produces:
  ```ts
  export type BannerId = 'noCctk' | 'noAdmin' | 'noDcm' | 'optimizer';
  export function bannersFor(s: AppState): BannerId[];   // kolejność: noAdmin, noCctk, optimizer, noDcm
  export const BANNER_TEXT: Record<BannerId, { title: string; body: string; action?: string }>;
  ```
  Teksty: `noCctk` „Dell Command | Configure not found” / „Mode switching is unavailable. Install Dell Command | Configure.”; `noAdmin` „Not running as administrator” / „Restart MyPrecision as administrator.”; `noDcm` „Dell Command | Monitor not found” / „CPU temperature and fan speeds are unavailable.”; `optimizer` „Dell Optimizer is running” / „Make sure Dynamic Charge is off — otherwise Optimizer may override your profile.” z akcją „Don’t show again” (`dismissOptimizerWarning`).

- [ ] **Step 1: Testy**
  ```ts
  test('optimizer banner hidden after dismissal', ...)   // optimizerRunning true + optimizerWarningDismissed true → brak 'optimizer'
  test('noCctk disables mode controls', ...)            // w Flyout: SegmentedControl disabled
  test('order is noAdmin, noCctk, optimizer, noDcm', ...)
  ```
- [ ] **Step 2:** FAIL → **Step 3:** implementacja (flyout pokazuje tylko pierwszy baner, pełne okno wszystkie; toasty znikają po 5 s, `role="status"`) → **Step 4:** PASS.
- [ ] **Step 5: Commit** — `feat(ui): error banners and toasts`

---

### Task 15: Weryfikacja na urządzeniu i instalator

**Files:**
- Create: `docs/superpowers/verification/2026-10-09-stage1.md` (wyniki)
- Modify: `src-tauri/src/platform/wmi_dcim.rs` / `crates/core/src/dcim.rs` tylko jeśli porównanie z HWiNFO tego wymaga

- [ ] **Step 1: Build** — `npm run tauri build` → instalator NSIS w `target/release/bundle/nsis/`. Zainstaluj (użytkownik zatwierdza UAC).
- [ ] **Step 2: Profile i tryby** — każdy z 3 profili i 4 trybów z flyoutu i z menu; po każdym `cctk --PrimaryBattChargeCfg` / `--ThermalManagement` (terminal admina) zgadza się z UI. Zmiana z zewnątrz (`Bateria-DOM.bat`) widoczna w ikonie w ≤ 30 s.
- [ ] **Step 3: HWiNFO** — przy otwartym pełnym oknie 5 min pracy mieszanej; zapisz pary (MyPrecision CPU, oba surowe odczyty DCIM z `--probe`, HWiNFO „CPU Package”). Jeśli maksimum odbiega > 2 °C, a jeden z duplikatów pasuje, zmień `parse_dcim` na ten indeks (z testem) i zapisz decyzję w dokumencie weryfikacji.
- [ ] **Step 4: Budżety** — RAM procesu `myprecision.exe` po 5 min w tle (Menedżer zadań, „Pamięć (prywatny zestaw roboczy)”) < 15 MB i brak `msedgewebview2` należącego do aplikacji; czas od kliknięcia do pełnego flyoutu < 300 ms (nagranie ekranu 60 fps lub `performance.now()` w `main.ts` względem znacznika czasu przekazanego w URL). Wynik ponad budżet → zapisz i zgłoś użytkownikowi, nie zmieniaj architektury samodzielnie.
- [ ] **Step 5: GPU** — przy bezczynności i otwartym oknie przez 2 min `--probe` pokazuje `Sleeping`; GPU-Z/HWiNFO nie pokazuje wybudzeń wywołanych przez aplikację.
- [ ] **Step 6: Autostart** — wyloguj/zaloguj na baterii → aplikacja startuje.
- [ ] **Step 7: Commit** — `docs: stage 1 on-device verification results`
