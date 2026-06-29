# LoAr (Local Archive Utility)

`LoAr`은 로컬에 흩어져 있는 여러 소프트웨어 개발 프로젝트 및 디렉토리들을 단일 백업 저장소로 빠르고 안전하게 동기화하고 백업하기 위한 **Rust 기반의 로컬 아카이브 도구**입니다.

이 도구는 증분 백업(Incremental Skip), 일방향 동기화(One-way Sync), AES-256-GCM 파일 암호화, 메타데이터 인덱싱(SQLite) 기능을 결합하여 스토리지 공간을 최적화하고 보안을 강화합니다.

---

## 주요 특징 (Key Features)

1. **초고속 증분 백업 (Incremental Skip)**
   - 파일 크기, 최종 수정일자(mtime), 그리고 SHA-256 해시를 비교하여 변경되지 않은 파일은 백업 복사(및 암호화)를 자동으로 생략합니다.
   - 전송 연산(I/O)을 극도로 절약하여 기가바이트급의 리포지토리도 수초 이내에 동기화가 완료됩니다.
2. **일방향 동기화 및 찌꺼기 청소 (One-way Sync & Auto Cleanup)**
   - 원본 리포지토리에서 파일이 삭제된 경우, 백업 타겟 드라이브 내에서도 즉시 영구 삭제를 실행하여 백업 저장소 상태를 로컬과 완벽히 동기화합니다.
   - 파일 삭제 후 백업 저장소 내에 빈 폴더가 남거나 macOS 메타데이터(`.DS_Store`) 찌꺼기만 남은 빈 디렉토리는 깊이 우선 탐색(DFS) 알고리즘으로 자동 추적하여 흔적 없이 완전히 제거합니다.
3. **AES-256-GCM 안전 암호화**
   - 백업 대상 리포지토리마다 독립적인 암호화 여부(`encrypt = true`)를 설정할 수 있습니다.
   - Argon2id 대칭 키 유도 알고리즘과 AES-256-GCM 암호화 엔진을 탑재하여, 파일명 난독화 및 파일 콘텐츠 전체를 빈틈없이 기밀 보관합니다.
4. **글로벌 백업 데이터베이스 (SQLite DB)**
   - 백업 타겟 폴더 최상위에 SQLite 파일(`loar.db`)을 두고 데이터 이력과 개별 파일 세션 인덱스를 완벽하게 관리합니다.
   - 백업 레코드가 삭제될 때마다 `VACUUM` 명령을 자동 구동하여 데이터베이스 물리 디스크 크기를 항상 최적의 크기로 유축합니다.
5. **대화형 TUI 및 자동화 CLI 듀얼 모드**
   - 편리하게 메뉴를 타고 백업/복원을 진행할 수 있는 대화형 터미널 메뉴(TUI)를 지원합니다.
   - Cron이나 외부 스케줄러 자동 백업 작업에 적합한 완전 비대화형 CLI 파라미터 모드를 함께 지원합니다.

---

## 설치 방법 (Installation)

`LoAr`은 네 가지 방식으로 설치할 수 있습니다: 원클릭 스크립트 설치(권장), Homebrew(macOS용), Scoop(Windows용), 직접 패키지 다운로드, 또는 소스코드 직접 빌드.

### 1. 원클릭 스크립트 설치 (모든 플랫폼 - 권장)
한 줄의 명령어로 즉시 `LoAr`을 설치할 수 있습니다. 스크립트가 실행 기기의 OS와 아키텍처를 자동으로 감지하여 최신 릴리즈 바이너리를 경로에 등록해 줍니다.

*   **Linux / macOS**:
    ```bash
    curl -fsSL https://raw.githubusercontent.com/cavecafe-cc/homebrew-tap/main/install.sh | sh
    ```

*   **Windows (PowerShell)**:
    ```powershell
    irm https://raw.githubusercontent.com/cavecafe-cc/homebrew-tap/main/install.ps1 | iex
    ```

### 2. Homebrew 사용 (macOS)
macOS 환경에서는 공식 Homebrew 탭을 통해 간편하게 설치할 수 있습니다:

```bash
brew tap cavecafe-cc/homebrew-tap
brew install loar
```

### 3. Scoop 사용 (Windows)
Windows 환경에서는 `Scoop` 패키지 매니저를 통해 한 줄로 간편하게 최신 버전을 설치할 수 있습니다:

```powershell
scoop install https://raw.githubusercontent.com/cavecafe-cc/local-archive/main/scoop/loar.json
```

### 4. APT 패키지 매니저 사용 (Linux / Debian / Ubuntu)
Cloudflare R2에 호스팅되는 공식 APT 저장소를 추가하여 `LoAr`을 패키지로 관리할 수 있습니다:

```bash
# 1. 저장소 공개 GPG 키 등록
curl -fsSL https://bin.cavecafe.cc/downloads/loar/apt/gpg.key | sudo gpg --dearmor -o /etc/apt/trusted.gpg.d/cavecafe-cc.gpg

# 2. APT 저장소 소스 리스트 추가
echo "deb [arch=amd64,arm64] https://bin.cavecafe.cc/downloads/loar/apt stable main" | sudo tee /etc/apt/sources.list.d/loar.list

# 3. 패키지 인덱스 갱신 및 설치
sudo apt-get update
sudo apt-get install loar
```

### 5. Snapcraft 사용 (Linux 공통)
`LoAr`은 classic confinement 권한을 지닌 스냅 패키지로 공식 등재되어 있습니다. Snap 환경이 완비된 모든 리눅스 배포판에서 아래 명령어로 설치할 수 있습니다:

```bash
sudo snap install loar --classic
```

### 6. 직접 패키지 다운로드 (빌드된 바이너리)
Cloudflare R2에 호스팅되는 빌드 완료 바이너리를 직접 다운로드할 수 있습니다. 특정 버전을 다운로드하고 싶으신 경우 URL 내의 `latest` 부분을 해당 버전명(예: `v1.0.43`)으로 변경하여 다운로드하십시오.

*   **macOS (Apple Silicon arm64)**:
    [loar-macos-arm64-latest.tar.gz](https://bin.cavecafe.cc/downloads/loar/loar-macos-arm64-latest.tar.gz)
*   **Linux (Debian x86_64)**:
    [loar-linux-x86_64-latest.deb](https://bin.cavecafe.cc/downloads/loar/loar-linux-x86_64-latest.deb)
*   **Linux (Debian ARM64)**:
    [loar-linux-arm64-latest.deb](https://bin.cavecafe.cc/downloads/loar/loar-linux-arm64-latest.deb)
*   **Windows (x86_64)**:
    [loar-windows-x86_64-latest.zip](https://bin.cavecafe.cc/downloads/loar/loar-windows-x86_64-latest.zip)
*   **Windows (ARM64)**:
    [loar-windows-arm64-latest.zip](https://bin.cavecafe.cc/downloads/loar/loar-windows-arm64-latest.zip)

#### Debian 계열 설치 단계 (.deb)
```bash
# 예시: 최신 x86_64 패키지 다운로드 및 설치
curl -LO https://bin.cavecafe.cc/downloads/loar/loar-linux-x86_64-latest.deb
sudo dpkg -i loar-linux-x86_64-latest.deb
```

#### Windows 계열 설치 단계 (.zip)
압축 파일(.zip)을 해제한 후, 시스템 PATH 환경 변수에 등록된 폴더(예: `C:\Windows\System32` 또는 사용자 지정 실행용 폴더)로 `loar.exe` 파일을 이동시키십시오.

---

### 3. 소스코드 빌드 설치 (Cargo)
소스코드에서 직접 컴파일하여 빌드하길 원하시는 경우, 시스템에 [Rust / Cargo](https://rustup.rs/)가 설치되어 있어야 합니다.

```bash
# 1. 프로젝트의 'src' 디렉토리로 이동
cd src

# 2. release 프로파일로 빌드 실행
cargo build --release

# 3. 컴파일 완료된 바이너리를 PATH 환경변수 폴더에 복사
cp target/release/loar ~/.cargo/bin/loar
```

---

## 설정 방법 (Configuration)

`LoAr`은 홈 디렉토리 하위의 `~/.loar/loar.toml` 파일을 통해 전역 백업 타겟 경로, 제외 필터, 동기화 대상 리포지토리들을 설정합니다.

### 설정 양식 예시 (`~/.loar/loar.toml`)

```toml
# 백업 대상 파일들이 복사되어 저장될 글로벌 백업 드라이브 경로
target_dir = "/Volumes/Backup/LoAr"

# 모든 리포지토리에 전역으로 적용할 백업 제외 규칙 (Gitignore 문법)
global_exclude = [
    ".DS_Store",
    "node_modules/",
    "target/",
    "bin/",
    "obj/",
    ".idea/",
    ".vscode/",
    "DerivedData/",
    "*.xcodeproj/xcuserdata/",
    "*.xcworkspace/xcuserdata/",
    "*.xcodeproj/project.xcworkspace/xcuserdata/",
    "ephemeral/",
    "Pods/",
    "gradle-wrapper.jar",
    "GeneratedPluginRegistrant.*",
    "Generated.xcconfig",
    "generated_plugin*",
]

# 백업할 개별 리포지토리 목록 정의
[[repositories]]
name = "LoAr"                                 # 백업 폴더 구분 이름
path = "/Users/username/Projects/LoAr"        # 로컬 소스 디렉토리 절대 경로
encrypt = false                              # 암호화 여부
one_way_sync = true                          # 로컬에서 지워진 파일 동시 삭제 여부

[[repositories]]
name = "Super7"
path = "/Users/username/Repo/GitHub/Super7"
encrypt = false
one_way_sync = true

[[repositories]]
name = "Private-Repo"
path = "/Users/username/Projects/Secret"
encrypt = true                               # 암호화 활성화
one_way_sync = true
```

---

## 기본 사용 설명 (Usage Guide)

### 1. 대화형 터미널 UI 모드 (TUI Mode)
터미널에서 아무런 인수 없이 `loar`를 입력하면 인터랙티브 TUI 제어판으로 진입합니다.

```bash
$ loar
```
진입 후 방향키와 Space(다중 선택), Enter 키를 사용해 다음 작업을 직관적으로 수행할 수 있습니다.
- **1. List Repositories**: 현재 등록된 동기화 리포지토리 정보 조회
- **2. Run Backup**: 특정 혹은 전체 리포지토리 증분 백업 가동
- **3. Register Repository**: 신규 리포지토리 정보 대화형 입력 등록
- **4. Restore Backup**: 특정 아카이브 세션을 지정 경로로 복원
- **5. Exit**: 프로그램 종료

---

### 2. 비대화형 명령 모드 (CLI Mode)

#### 1) 전체 리포지토리 백업 실행
현재 등록된 모든 리포지토리의 증분 백업을 차례대로 한 번에 가동합니다. (Cron 스케줄러 등록에 매우 적합합니다.)
```bash
$ loar run --all
```

#### 2) 특정 리포지토리만 백업 실행
특정 리포지토리 이름(TOML의 `name`)을 지정해 백업을 수행합니다.
```bash
$ loar run --repo owl
```
*암호화된 리포지토리를 백업/복원 시 패스워드 입력을 건너뛰고 비대화형으로 실행하고 싶다면 `--password` 플래그를 함께 기입합니다:*
```bash
$ loar run --repo Private-Repo --password "your_secure_password"
```

#### 3) 백업 상태 및 이력 조회
등록된 리포지토리의 이름, 로컬 경로, 암화화 상태, 동기화 모드 및 마지막 백업 실행 시점을 확인합니다.
```bash
$ loar status
```

#### 4) 리포지토리 수동 등록
새로운 백업 리포지토리를 명령어로 직접 등록합니다.
```bash
$ loar register --name "WWW" --path "/path/to/www" --encrypt
```
- `--name`: 백업 저장 시 구분할 이름 (필수)
- `--path`: 백업할 소스 디렉토리 경로 (필수)
- `--encrypt`: 백업 파일 암호화 모드 켜기 (선택)
- `--no-sync`: One-way Sync(일방향 동기화) 비활성화 및 스냅샷 이력 아카이빙 유지 (선택)

#### 5) 백업 복원 (Restore)
특정 백업 세션을 로컬 경로로 안전하게 다운로드 복구합니다.
```bash
$ loar restore --repo owl --dest /Users/username/Restored/owl
```

#### 6) 리포지토리 등록 해제 (Unregister)
등록된 리포지토리를 해제하며, 해당 리포지토리의 데이터베이스 백업 이력 및 백업 저장소에 동기화된 모든 백업 파일을 안전하게 삭제합니다.
```bash
$ loar unregister --repo Super7
```
- `--repo`: 등록 해제할 리포지토리 이름 (필수)

---

## 백업 제외 필터 관리 (`.loar.ignore`)

`LoAr`은 파일 스캔 시 전역 제외 설정(`global_exclude`)을 거친 후, 개별 리포지토리 루트에 자동 생성되는 `.loar.ignore` 필터 규칙을 최우선 적용합니다.

### 1. 자동 무시 동작 방식
- 리포지토리 최초 백업 시 소스 폴더 루트에 `.loar.ignore` 파일이 자동으로 생성됩니다.
- 이 파일 내에 Gitignore 형식 패턴으로 기재된 하위 파일 및 폴더들은 아카이브 대상에서 완전히 제외됩니다.

### 2. 기본 ignore 템플릿 포함 목록
기본적으로 다음 파일들은 백업 용량 최적화 및 빌드 종속성 파일 방지를 위해 기본 누락 차단됩니다.
- **OS 시스템 메타데이터**: `.DS_Store`, `Thumbs.db` 등
- **빌드 종속성 및 바이너리**: `node_modules/`, `target/`, `bin/`, `obj/`, `*.o`, `*.exe` 등
- **iOS 및 CocoaPods 빌드 아티팩트**: `DerivedData/`, `Pods/`, `ephemeral/` 등
- **빌드 임시 자동 생성 도구**: `gradle-wrapper.jar`, `GeneratedPluginRegistrant.*`, `Generated.xcconfig`, `generated_plugin*` 등
- **IDE 설정**: `.idea/`, `.vscode/` 등
