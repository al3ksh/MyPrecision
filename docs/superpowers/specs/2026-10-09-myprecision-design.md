# MyPrecision — specyfikacja (etap 1)

Data: 2026-10-09
Urządzenie docelowe: Dell Precision 5560 (BIOS 1.47.0), Windows 11 Pro

## 1. Cel

Lekka aplikacja "premium" zastępująca Dell Optimizer i SupportAssist w codziennym użyciu:
przełączanie trybów ładowania baterii i chłodzenia jednym kliknięciem oraz mały podgląd
czujników ("mini HWiNFO"). Po wdrożeniu użytkownik może odinstalować Dell Optimizer
i SupportAssist.

### Kontekst użytkownika
- Laptop przez większość czasu pracuje na zasilaczu w domu, ale na uczelni potrzebne jest
  pełne 100% baterii.
- Bateria ma już 13,6% zużycia (67 914 mWh fabrycznie → 58 709 mWh), więc priorytetem jest
  ograniczenie czasu spędzanego na 100% i w wysokiej temperaturze.

### Kryteria sukcesu
- Zmiana trybu baterii / chłodzenia: 1 klik z zasobnika, potwierdzona odczytem z BIOS.
- W tle (okno schowane): < 15 MB RAM, ~0% CPU.
- Okienko z zasobnika otwiera się w < 300 ms, animacje płynne (60+ fps).
- Odczyty temperatur zgodne z HWiNFO (±2 °C).

## 2. Zakres

### Etap 1 (ta specyfikacja)
- Ikona w zasobniku + okienko (flyout) + pełne okno.
- Tryby ładowania baterii, tryby chłodzenia.
- Czujniki: CPU (temperatura, obciążenie), GPU (temperatura, obciążenie), wentylatory (RPM), RAM,
  bateria (%, moc, stan).
- Zdrowie baterii: pojemność fabryczna / obecna, zużycie %, cykle, historia dzienna.
- Wykresy ostatnich 30 minut w pełnym oknie.
- Autostart z uprawnieniami administratora.

### Etap 2 (osobna specyfikacja, później)
- Zakładka ustawień BIOS (podświetlenie klawiatury, Fn lock, USB PowerShare, wake on AC…).
- Aktualizacje sterowników i BIOS przez Dell Command | Update CLI.
- Informacje o urządzeniu i gwarancji.

### Poza zakresem
- Ręczne krzywe wentylatorów, undervolting, overclocking (niedostępne / ryzykowne na Dellu).
- Funkcje "AI" Dell Optimizera (ExpressResponse, ExpressSign-in, Intelligent Audio).
- Diagnostyka sprzętu (zostaje wbudowane ePSA: F12 → Diagnostics).

## 3. Technologia

- **Tauri 2** (Rust + WebView2).
- **Frontend:** Svelte 5 + TypeScript + Vite. Bez bibliotek UI; własne komponenty w stylu Graphite.
- **Zależności zewnętrzne (oficjalne narzędzia Della, bez GUI i bez procesów w tle):**
  - Dell Command | Configure (`cctk.exe`) — zainstalowane, v5.2.1.
  - Dell Command | Monitor — do doinstalowania; dostarcza temperatury i obroty wentylatorów przez WMI
    (`root\dcim\sysman`).

## 4. Styl wizualny — "Graphite"

- Ciemny motyw w stylu Windows 11: tło okna z efektem Mica (pełne okno) / Acrylic (flyout),
  powierzchnie `#202020` / `#2d2d2d`, obramowania `#3a3a3a`, tekst `#f3f3f3` / `#9d9d9d`.
- Jeden kolor akcentu: `#4cc2ff` (tekst na akcencie `#003049`).
- Font: Segoe UI Variable; liczby z `font-variant-numeric: tabular-nums`.
- Zaokrąglenia 8 px (kontrolki), 12 px (karty), flyout 14 px.
- Ruch: przełączniki segmentowe z przesuwanym wskaźnikiem (~180 ms, ease-out), liczby płynnie
  interpolowane, wejście flyoutu: przesunięcie 8 px + fade (~150 ms). `prefers-reduced-motion`
  wyłącza animacje.
- Kolor ikony w zasobniku zależy od trybu baterii: Dom = akcent, Uczelnia = biały,
  Przechowywanie = szary.

## 5. Architektura

```
┌──────────────────────── proces MyPrecision (admin) ────────────────────────┐
│ Rust core                                                                   │
│  dell::cctk ── cctk.exe (get/set BIOS)                                      │
│  sensors::{battery,cpu,gpu,fans,mem} ── WMI / PDH / NVML                    │
│  poller ── harmonogram odczytów (1 s / 30 s)                                │
│  history ── bufor 30 min (RAM) + battery-health.json (dziennie)             │
│  config ── %APPDATA%\MyPrecision\config.json                                │
│  tray ── ikona + menu                                                       │
│  autostart ── Harmonogram zadań                                             │
│        │  komendy Tauri (invoke) / zdarzenia (emit "telemetry")             │
│ WebView2 (tylko gdy okno widoczne): Svelte UI — flyout lub pełne okno       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.1 Moduły Rust

| Moduł | Odpowiedzialność | Interfejs |
|---|---|---|
| `dell::cctk` | Lokalizacja `cctk.exe`, uruchamianie (bez okna konsoli), parsowanie wyjścia | `get_charge_cfg() -> ChargeCfg`, `set_charge_cfg(ChargeCfg)`, `get_thermal() -> ThermalMode`, `set_thermal(ThermalMode)` |
| `sensors::battery` | %, stan (ładowanie / rozładowanie / bypass), moc W, pojemności, cykle | `read() -> BatterySnapshot` |
| `sensors::cpu` | Obciążenie (PDH / `GetSystemTimes`), temperatura (Dell Command \| Monitor) | `read() -> CpuSnapshot` |
| `sensors::gpu` | NVML: temperatura, obciążenie; tylko gdy GPU już aktywne | `read() -> GpuSnapshot` (stan `Asleep`/`Active`/`Unavailable`) |
| `sensors::fans` | RPM wentylatorów (Dell Command \| Monitor) | `read() -> Vec<FanReading>` |
| `sensors::mem` | Zajęty / całkowity RAM | `read() -> MemSnapshot` |
| `poller` | Wątek odczytów; tryb `Active` (1 s, wszystkie czujniki) i `Idle` (30 s, bateria + tryby) | `set_mode(PollMode)`; emituje `Telemetry` |
| `history` | Bufor kołowy 1800 próbek; dzienny zapis zdrowia baterii | `push`, `range(minutes)`, `battery_health_log()` |
| `config` | Progi trybów, preferencje (autostart, jednostki) | `load()`, `save()` |
| `tray` | Ikona (kolor wg trybu), tooltip (%, tryb, temp CPU), menu | — |
| `autostart` | Tworzenie / usuwanie zadania "MyPrecision" (At logon, Highest privileges) przez `schtasks` | `enable()`, `disable()`, `is_enabled()` |

Każdy moduł czujników jest niezależny i zwraca `Option`/stan "niedostępne" zamiast błędu —
brak jednego źródła nie blokuje reszty.

### 5.2 Typy domenowe

```rust
enum ChargeCfg { Standard, Adaptive, PrimAcUse, Express, Custom { start: u8, stop: u8 } }
enum ThermalMode { Optimized, Cool, Quiet, UltraPerformance }
enum BatteryProfile { Home, Campus, Storage }   // mapowane na ChargeCfg przez config
```

Domyślne profile:
- Dom: `Custom { start: 75, stop: 80 }`
- Uczelnia: `Standard`
- Przechowywanie: `Custom { start: 50, stop: 60 }`

Aktywny profil jest wyznaczany z odczytu BIOS (nie z pamięci aplikacji), więc zmiana dokonana
innym narzędziem jest widoczna. Ustawienie spoza profili wyświetla się jako "Własne (x–y%)".

### 5.3 Komendy i zdarzenia (Rust ↔ UI)

- `invoke("get_state") -> AppState` (tryby, statyczne dane baterii, dostępność źródeł)
- `invoke("set_battery_profile", { profile })` → po zapisie ponowny odczyt z BIOS i zwrot stanu
- `invoke("set_thermal_mode", { mode })` → j.w.
- `invoke("get_history", { minutes })`
- `invoke("set_autostart", { enabled })`
- zdarzenie `telemetry` co 1 s w trybie `Active`

### 5.4 Cykl życia okien

- Start: tylko ikona w zasobniku, brak WebView, poller w trybie `Idle`.
- Lewy klik ikony: tworzy okno flyout (bez ramki, zawsze na wierzchu, nad zasobnikiem),
  poller → `Active`. Utrata fokusu zamyka flyout i niszczy WebView, poller → `Idle`.
- "Otwórz pełne okno" (flyout lub menu): okno 960×640 z Mica; zamknięcie niszczy WebView.
- Prawy klik ikony: natywne menu — profile baterii, tryby chłodzenia, Otwórz, Autostart, Zamknij.
- Jedna instancja aplikacji (`tauri-plugin-single-instance`).

### 5.5 GPU (Optimus)

RTX w trybie Optimus przechodzi w D3cold; zapytanie NVML może ją wybudzić. Przed odczytem
sprawdzany jest stan zasilania urządzenia (SetupAPI / `DEVPKEY_Device_PowerData`);
jeśli GPU śpi → stan `Asleep`, bez wywołań NVML. NVML ładowane dynamicznie (`nvml.dll`) —
brak sterownika = `Unavailable`.

### 5.6 Uprawnienia

`cctk` i odczyt temperatur wymagają administratora. Manifest aplikacji: `requireAdministrator`.
Autostart przez Harmonogram zadań z "Highest privileges" omija monit UAC przy logowaniu.

## 6. Obsługa błędów

| Sytuacja | Zachowanie |
|---|---|
| Brak `cctk.exe` | Sekcje trybów wyłączone, baner z linkiem do Dell Command \| Configure |
| `cctk` zwraca błąd / hasło BIOS | Toast z treścią błędu po polsku, stan odczytany ponownie |
| Brak Dell Command \| Monitor | Temperatury CPU i wentylatory: "—" + podpowiedź instalacji |
| GPU śpi / brak sterownika | "Uśpiona" / "Niedostępna" |
| Dell Optimizer z włączonym Dynamic Charge | Baner ostrzegający, że Optimizer może nadpisywać profil |
| Brak uprawnień admina | Baner "Uruchom ponownie jako administrator" |

## 7. Dane na dysku

`%APPDATA%\MyPrecision\`
- `config.json` — progi profili, preferencje.
- `battery-health.json` — maks. 1 wpis na dzień: data, full charge capacity, design capacity, cykle.

## 8. Testy

- **Jednostkowe (Rust):** parsowanie wyjścia `cctk` (wszystkie warianty `ChargeCfg`, `ThermalMode`,
  błędy), mapowanie profil ↔ `ChargeCfg`, obliczanie zużycia i stanu bypass, bufor historii,
  zapis/odczyt dziennika zdrowia. Źródła danych za traitami, testy na danych przykładowych.
- **Frontend:** `svelte-check` + testy komponentów przełączników (Vitest).
- **Ręczne na urządzeniu:** przełączenie każdego profilu i trybu z weryfikacją
  `cctk --PrimaryBattChargeCfg` / `--ThermalManagement`, porównanie temperatur z HWiNFO,
  pomiar RAM w tle (Menedżer zadań), czas otwarcia flyoutu, zachowanie GPU (czy nie jest wybudzane).

## 9. Wymagania wstępne (po stronie użytkownika)

1. Zainstalować Dell Command | Monitor.
2. Wyłączyć Dynamic Charge w Dell Optimizer (docelowo odinstalować Optimizer i SupportAssist).

## 10. Do zweryfikowania na starcie implementacji

- Dokładne nazwy opcji i wartości `cctk` 5.2.1 na tym modelu (`--PrimaryBattChargeCfg`,
  `--ThermalManagement`) — `cctk --help` z uprawnieniami admina.
- Klasy WMI Dell Command | Monitor dla temperatury CPU i wentylatorów na 5560.
