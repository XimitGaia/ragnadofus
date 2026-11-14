//! Runner do GATE do ragnadofus — a forma canônica de rodar a verificação ("local ==
//! CI", padroes §4 / RFC 0006 §4.5).
//!
//! `cargo run -p xtask ci` roda, em ordem e parando no primeiro erro:
//!   1. Grafo unidirecional (client/server → shared; client ↮ server; sem ciclos).
//!   2. Desacoplamento do renderer: build ISOLADO do `server` + scan do grafo de build —
//!      `bevy_render` NÃO pode estar no grafo do server (nem, por consequência, do
//!      shared). Isolado de propósito: o resolver v2 escopa features aos pacotes
//!      construídos, então o `bevy` do server compila sem render (num build de workspace
//!      inteiro a unificação de features ligaria o render por causa do client — por isso
//!      NÃO usamos `cargo tree`/`cargo metadata`, que são unificados).
//!   3. `bevy_render` PRESENTE no client (prova positiva de que o render está acoplado lá).
//!   4. `cargo build --workspace --all-targets`.
//!   5. `cargo test --workspace`.
//!   6. `cargo clippy --workspace --all-targets -- -D warnings` (unwrap/expect negados).
//!   7. `cargo deny check` (só se instalado; senão avisa e pula — nunca passa em silêncio).
//!
//! Forbidden-path (RFC 0006 §4.6): este runner É o gate. Mudanças que relaxem o gate
//! para "passar" são reward-hacking — devem ser justificadas, não silenciosas.

use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => ci(),
        other => {
            eprintln!("uso: cargo run -p xtask ci");
            bail!("tarefa desconhecida: {other:?}");
        }
    }
}

fn ci() -> Result<()> {
    banner("GATE ragnadofus");

    step(1, "grafo unidirecional (client/server → shared; client ↮ server)");
    check_graph_direction()?;

    step(2, "desacoplamento do renderer: server (e shared) SEM bevy_render");
    check_server_renderless()?;

    step(3, "render acoplado ao client (bevy_render presente)");
    check_client_has_render()?;

    step(4, "cargo build --workspace --all-targets");
    run("cargo", &["build", "--workspace", "--all-targets"])?;

    step(5, "cargo test --workspace");
    run("cargo", &["test", "--workspace"])?;

    step(6, "cargo clippy (-D warnings; unwrap/expect negados)");
    run("cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;

    step(7, "cargo deny check (opcional)");
    cargo_deny_optional()?;

    println!("\n\x1b[32m✅ GATE VERDE\x1b[0m — grafo íntegro, build e testes passam.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Passo 1 — direção do grafo (via cargo metadata; arestas internas são explícitas,
// logo imunes à unificação de features).
// ---------------------------------------------------------------------------

fn check_graph_direction() -> Result<()> {
    let md = metadata()?;
    let id_to_name = package_names(&md)?;
    let edges = resolve_edges(&md)?;
    let members = member_ids(&md, &id_to_name)?;

    // Regra P1: shared é folha do grafo interno; client e server não se enxergam.
    let rules: [(&str, &[&str]); 3] = [
        ("shared", &["client", "server"]),
        ("client", &["server"]),
        ("server", &["client"]),
    ];
    for (crate_name, forbidden) in rules {
        let start = members
            .get(crate_name)
            .with_context(|| format!("membro de workspace `{crate_name}` não encontrado"))?;
        let reachable = transitive_closure(start, &edges);
        for other in forbidden {
            let hit = reachable
                .iter()
                .any(|id| id_to_name.get(id).map(String::as_str) == Some(*other));
            if hit {
                bail!("VIOLAÇÃO de grafo: `{crate_name}` depende de `{other}` (proibido pela regra P1)");
            }
        }
    }
    println!("      ok: shared ↛ client/server · client ↮ server · sem ciclos");
    Ok(())
}

// ---------------------------------------------------------------------------
// Passo 2 — server renderless, provado por BUILD ISOLADO.
// `cargo build -p server --message-format=json`: as mensagens compiler-artifact
// listam exatamente os crates do grafo de build do server. Se `bevy_render` aparecer,
// o server não está desacoplado. Isolado => feature resolution só do server.
// ---------------------------------------------------------------------------

fn check_server_renderless() -> Result<()> {
    println!("      $ cargo build -p server (isolado, lendo o grafo de build)");
    let mut child = Command::new("cargo")
        .args(["build", "-p", "server", "--message-format=json-render-diagnostics"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("falha ao iniciar `cargo build -p server`")?;

    let stdout = child.stdout.take().context("sem stdout do cargo")?;
    let compiled = read_artifact_names(stdout)?;

    let status = child.wait().context("falha ao aguardar `cargo build -p server`")?;
    if !status.success() {
        bail!("`cargo build -p server` falhou ({status})");
    }

    if compiled.iter().any(|n| n == "bevy_render") {
        bail!("VIOLAÇÃO: `bevy_render` está no grafo de build do `server` — servidor NÃO é headless");
    }
    println!(
        "      ok: server compila sozinho sem bevy_render ({} crates no grafo)",
        compiled.len()
    );
    Ok(())
}

/// Lê linhas NDJSON do cargo e coleta os nomes dos `target` de mensagens
/// `compiler-artifact`.
fn read_artifact_names<R: std::io::Read>(reader: R) -> Result<HashSet<String>> {
    use std::io::BufRead;
    let mut names = HashSet::new();
    for line in std::io::BufReader::new(reader).lines() {
        let line = line.context("erro lendo stdout do cargo")?;
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if value["reason"].as_str() == Some("compiler-artifact") {
            if let Some(name) = value["target"]["name"].as_str() {
                names.insert(name.to_string());
            }
        }
    }
    Ok(names)
}

// ---------------------------------------------------------------------------
// Passo 3 — render acoplado ao client. Aqui a unificação não atrapalha: só queremos
// confirmar que bevy_render ESTÁ presente. `cargo tree -i` sai 0 se encontrar.
// ---------------------------------------------------------------------------

fn check_client_has_render() -> Result<()> {
    let status = Command::new("cargo")
        .args(["tree", "-p", "client", "-i", "bevy_render"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("falha ao rodar `cargo tree -p client`")?;
    if !status.success() {
        bail!("`bevy_render` NÃO está no fecho do `client` — render não acoplado");
    }
    println!("      ok: client acopla bevy_render");
    Ok(())
}

// ---------------------------------------------------------------------------
// Passo 7 — cargo-deny opcional.
// ---------------------------------------------------------------------------

fn cargo_deny_optional() -> Result<()> {
    let present = Command::new("cargo")
        .args(["deny", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !present {
        println!("      \x1b[33mAVISO\x1b[0m: cargo-deny não instalado — supply-chain PULADO (não é passe silencioso)");
        return Ok(());
    }
    run("cargo", &["deny", "check"])
}

// ---------------------------------------------------------------------------
// Helpers de metadata/grafo.
// ---------------------------------------------------------------------------

fn metadata() -> Result<serde_json::Value> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()
        .context("falha ao rodar `cargo metadata`")?;
    if !out.status.success() {
        bail!("`cargo metadata` falhou: {}", String::from_utf8_lossy(&out.stderr));
    }
    serde_json::from_slice(&out.stdout).context("JSON inválido de `cargo metadata`")
}

/// Mapa id-de-pacote → nome, para todos os pacotes resolvidos.
fn package_names(md: &serde_json::Value) -> Result<HashMap<String, String>> {
    let packages = md["packages"].as_array().context("campo `packages` ausente")?;
    let mut map = HashMap::new();
    for p in packages {
        let id = p["id"].as_str().context("pacote sem `id`")?;
        let name = p["name"].as_str().context("pacote sem `name`")?;
        map.insert(id.to_string(), name.to_string());
    }
    Ok(map)
}

/// Arestas do grafo resolvido: id → ids dependidos.
fn resolve_edges(md: &serde_json::Value) -> Result<HashMap<String, Vec<String>>> {
    let nodes = md["resolve"]["nodes"].as_array().context("`resolve.nodes` ausente")?;
    let mut edges = HashMap::new();
    for n in nodes {
        let id = n["id"].as_str().context("node sem `id`")?;
        let deps = n["dependencies"]
            .as_array()
            .context("node sem `dependencies`")?
            .iter()
            .filter_map(|d| d.as_str().map(str::to_string))
            .collect();
        edges.insert(id.to_string(), deps);
    }
    Ok(edges)
}

/// Nome de crate → id, restrito aos membros do workspace.
fn member_ids(
    md: &serde_json::Value,
    id_to_name: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let members = md["workspace_members"].as_array().context("`workspace_members` ausente")?;
    let mut map = HashMap::new();
    for m in members {
        let id = m.as_str().context("membro sem id")?;
        if let Some(name) = id_to_name.get(id) {
            map.insert(name.clone(), id.to_string());
        }
    }
    Ok(map)
}

fn transitive_closure(start: &str, edges: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(id) = stack.pop() {
        if let Some(deps) = edges.get(&id) {
            for d in deps {
                if seen.insert(d.clone()) {
                    stack.push(d.clone());
                }
            }
        }
    }
    seen
}

// ---------------------------------------------------------------------------
// Execução de comandos + UI.
// ---------------------------------------------------------------------------

fn run(cmd: &str, args: &[&str]) -> Result<()> {
    println!("      $ {cmd} {}", args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("falha ao executar `{cmd}`"))?;
    if !status.success() {
        bail!("comando falhou ({status}): {cmd} {}", args.join(" "));
    }
    Ok(())
}

fn banner(title: &str) {
    println!("\n\x1b[1m━━━ {title} ━━━\x1b[0m");
}

fn step(n: u8, what: &str) {
    println!("\n\x1b[36m[{n}/7]\x1b[0m {what}");
}
