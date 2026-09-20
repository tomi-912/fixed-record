# リポジトリ作業指示

このファイルは、このリポジトリで作業するときに常に守るルールと技術上の制約を定義します。
英語版は [AGENTS.md](AGENTS.md) です。

## ドキュメントの同期

- `AGENTS.md` と `AGENTS.ja.md` は必ず同じ変更内で同時に更新します。片方だけを変更してはいけません。
- 両ファイルの意味を一致させます。解釈が曖昧な場合は英語版を正とします。
- `README.md` / `README.ja.md` と `DEVELOPMENT.md` / `DEVELOPMENT.ja.md` にも同じルールを適用します。
- 公開するRustドキュメントは英語を先、日本語を後の順で併記します。
- READMEはインストールと利用者向けの使い方に集中させます。
- 長期間適用するリポジトリルールと技術上の制約はAGENTSに置きます。
- 改善案、設計上の検討事項、リリース計画はDEVELOPMENTまたはそこからリンクしたGitHub Issuesに置きます。

## プロジェクト方針

- 現在のリリースは `0.1.0` です。
- 次の大きな目標は `1.0.0` です。
- `1.0.0` 公開前は、公開APIの命名と一貫性を最優先でレビューします。
- `1.0.0` の準備中は `0.1.0` との後方互換性を必須としません。互換用APIを増やすより、一貫したAPIを優先します。ただし、意図的な破壊的変更は記録します。
- 実行可能な改善案はGitHub Issuesで管理します。独立してレビュー可能な変更ごとに1つのIssueを作り、`1.0.0` のロードマップに関係する間はDEVELOPMENTからリンクします。
- 複数Issueの優先順位や進捗をボードまたはロードマップで確認したい場合はGitHub Projectsを使います。

## 命名

- 公開crate: `fixed-record`
- コード上のcrate名: `fixed_record`
- proc macro crate: `fixed-record-macros`
- attribute macro: `#[fixed_record]`
- repository: <https://github.com/tomi-912/fixed-record>
- license: MIT No Attribution License (`MIT-0`)

利用者は `fixed-record` だけに依存します。`fixed-record-macros` は `fixed-record` から再エクスポートする実装用crateとして扱います。生成コードは `::fixed_record::...` のpathを使います。

## Workspace構成

```text
crates/
  fixed-record/          # 公開library crate
  fixed-record-macros/   # proc macro実装
examples/
  basic/                 # 小さな実行example
  no-list/               # default-features = falseのtest fixture
```

API保証と挙動testは `crates/fixed-record/` に置きます。`examples/basic` は小さく利用者向けに保ちます。同一library package内のintegration testでは自packageの依存featureを切り替えにくいため、no-listのcompile-fail testは `examples/no-list` に置きます。

## 作業ルール

- dirty worktreeにあるユーザーの無関係な変更を維持します。
- ソースまたはドキュメントを変更したら原則としてcommitし、そこで止めます。pushはユーザーから明示的に依頼された場合だけ行います。
- 生成された `target/` の内容は検索、レビュー、commitの対象から除外します。
- 変更範囲を目的に集中させ、挙動上のリスクに応じてtestを追加します。
- 実質的な変更を完了する前にformat、test、Clippyを実行します。

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

- 生成APIまたはfeatureの挙動を変更した場合は、関連するexampleも実行します。

```bash
cargo run -p fixed-record-basic-example --bin fixed_record_usage
cargo run -p fixed-record-basic-example --bin macro_reexport
cargo run -p fixed-record-no-list-example
```

## 公開APIの制約

### 固定長レコードとzerocopy

- 生成レコードは `#[repr(C)]` とし、`zerocopy::FromBytes`、`IntoBytes`、`Immutable`、`KnownLayout` をderiveします。
- コピーするAPI (`parse`、`parse_str`、`to_bytes`) と、借用byte view API (`ref_from_bytes`、`as_bytes`、`as_mut_bytes`) を分離します。
- `ref_from_bytes` は入力サイズの完全一致を要求します。prefix variantは後続バイトを許容でき、短い入力を `Error::TooShort` に変換します。
- 文字列の幅は文字数ではなくバイト数です。
- 固定長レコードはbyte-orientedである一方、`&mut str` はvalid UTF-8を維持する必要があるため、mutable string viewを生成しません。
- zerocopyで安全に実現できる操作について、手書きのunchecked pointer cast APIを再導入しません。

### 生成List API

- defaultの `list` featureで `{StructName}List` の生成を制御します。`default-features = false` ではレコードとフィールド操作だけを生成します。
- 生成helperの可視性は入力レコードに合わせます。内部index型は常に非公開にします。
- Listのフィールドindexには型付きの `BTreeMap<Fixed<N>, Vec<usize>>` を使います。変更操作はunwind中も含めてindexの整合性を維持します。
- 失敗し得る検索名には `try_` prefixを使います。完全一致検索はフィールド幅を検証し、padding検索とprefix検索の意味を明確に区別します。
- 位置を変える操作では、必要に応じてindexを更新または再構築します。検索結果は現在のList順を維持します。
- 内部vectorのindexを安定したrecord IDとして公開しません。

### ReaderとWriter

- `Reader::new` と `Writer::new` のデフォルト区切りはLF (`\n`) です。
- 設定された区切りは最終レコードを含む各レコードの後ろに必要です。
- `Writer` はレコード書き込み時にNUL byteをspaceへ置き換えます。
- Reader/Writerの変更では、separator境界、不完全なレコード、I/O error、sequence checkをtestします。

## 生成APIのテスト

- integration testは `crates/fixed-record/tests/` に置きます。
- 不正なmacro入力、可視性、借用制約、生成されないAPIはcompile-fail testで確認します。
- `1.0.0` に向けて公開API名を変更するときは、生成コード、integration test、compile-fail fixture、example、README、rustdocを同じ変更内で更新します。
- 生成されるrustdocも公開APIレビューの対象として扱います。

## 公開

- 公開対象の両crateは、同一内容の `MIT-0` license fileと完全なcrates.io metadataを維持します。
- workspace buildではlocal crate、公開後のbuildではリリース済みversionを解決できるよう、`fixed-record` は `fixed-record-macros` にversion付きpath dependencyを使います。
- `fixed-record-macros` を先に公開し、そのversionがcrates.io indexへ反映されてから `fixed-record` を公開します。
- リリース前に両crateのpackage内容を確認し、`cargo publish --dry-run` を実行します。
- GitHub Actionsは `main` からrustdocを生成し、自動生成される `gh-pages` branchへ公開します。`gh-pages` を手作業で編集しません。
