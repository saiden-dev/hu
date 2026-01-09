#!/usr/bin/env python3
"""EKS Shell - Connect to Kubernetes pods easily."""

import subprocess
import sys
import signal
from concurrent.futures import ThreadPoolExecutor
from enum import Enum
from typing import Annotated, Optional

try:
    import typer
    from rich.console import Console
    from rich.table import Table
    from rich.panel import Panel
    from rich import print as rprint
except ImportError:
    print("Missing dependencies. Install with: pip install typer rich")
    sys.exit(1)

app = typer.Typer(
    name="eks-shell",
    help="🚀 EKS Shell - Connect to Kubernetes pods easily",
    add_completion=True,
    rich_markup_mode="rich",
)
console = Console()


class Environment(str, Enum):
    prod = "prod"
    dev = "dev"
    stg = "stg"


ENV_CONFIG = {
    "prod": {"cluster": "prod-eks", "emoji": "🔴", "long": "production"},
    "dev": {"cluster": "dev-eks", "emoji": "🟢", "long": "development"},
    "stg": {"cluster": "stg-eks", "emoji": "🟡", "long": "staging"},
}

POD_COLORS = ["red", "green", "yellow", "blue", "magenta", "cyan"]


def run_cmd(cmd: list[str], capture: bool = True, check: bool = True) -> subprocess.CompletedProcess:
    """Run a command and return the result."""
    return subprocess.run(cmd, capture_output=capture, text=True, check=check)


def detect_env() -> str | None:
    """Detect environment from current kubectl context."""
    try:
        result = run_cmd(["kubectl", "config", "current-context"])
        context = result.stdout.strip()
        for env in ENV_CONFIG:
            if env in context:
                return env
    except subprocess.CalledProcessError:
        pass
    return None


def check_aws_session() -> bool:
    """Check if AWS SSO session is active."""
    try:
        run_cmd(["aws", "sts", "get-caller-identity"])
        return True
    except subprocess.CalledProcessError:
        return False


def aws_sso_login():
    """Perform AWS SSO login."""
    subprocess.run(["aws", "sso", "login"], check=True)


def update_kubeconfig(cluster: str, region: str = "us-east-1"):
    """Update kubeconfig for the specified cluster."""
    run_cmd(["aws", "eks", "update-kubeconfig", "--name", cluster, "--region", region])


def get_pods(namespace: str, pattern: str) -> list[str]:
    """Get pods matching the pattern."""
    result = run_cmd(["kubectl", "get", "pod", "-n", namespace, "--no-headers"], check=False)
    if result.returncode != 0:
        return []
    pods = []
    for line in result.stdout.strip().split("\n"):
        if line and pattern in line:
            pods.append(line.split()[0])
    return pods


def display_pods(pods: list[str], env: str, env_emoji: str):
    """Display pods in a nice table."""
    table = Table(show_header=True, header_style="bold magenta", border_style="blue")
    table.add_column("#", style="cyan bold", width=4)
    table.add_column("Pod Name", style="white")
    table.add_column("Short ID", style="dim")

    for i, pod in enumerate(pods, 1):
        short_id = pod[-5:]
        table.add_row(str(i), pod, short_id)

    panel = Panel(
        table,
        title=f"{env_emoji} [bold]Matching Pods[/bold] ({env})",
        border_style="blue",
        padding=(1, 2),
    )
    console.print()
    console.print(panel)
    console.print()


def exec_into_pod(pod: str, namespace: str, env: str, pod_type: str, pod_num: int, env_emoji: str):
    """Execute into a pod with a custom prompt."""
    prompt_label = f"{env}-{pod_type}-{pod_num}"
    
    panel = Panel(
        f"[bold]{pod}[/bold]\n[dim]Prompt:[/dim] {env_emoji} [cyan]{prompt_label}[/cyan] $",
        title="🔗 [green]Connecting[/green]",
        border_style="green",
    )
    console.print(panel)
    console.print()

    subprocess.run([
        "kubectl", "exec", "-it", pod, "-n", namespace, "--",
        "env",
        f"EKS_ENV={env}",
        f"EKS_TYPE={pod_type}",
        f"EKS_POD_NUM={pod_num}",
        f"EKS_LABEL={prompt_label}",
        f"EKS_EMOJI={env_emoji}",
        "bash", "-c",
        'export PS1="${EKS_EMOJI} \\[\\033[1;36m\\]${EKS_LABEL}\\[\\033[0m\\] $ "; exec bash --norc --noprofile'
    ])


def tail_pod_log(pod: str, namespace: str, log_file: str, color: str):
    """Tail a log file from a single pod."""
    short_id = pod[-5:]
    try:
        proc = subprocess.Popen(
            ["kubectl", "exec", pod, "-n", namespace, "--", "tail", "-f", log_file],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        for line in proc.stdout:
            rprint(f"[{color}][{short_id}][/{color}] {line.rstrip()}")
    except Exception:
        pass


def tail_logs(pods: list[str], namespace: str, log_file: str):
    """Tail logs from all pods in parallel."""
    panel = Panel(
        f"[bold]{log_file}[/bold]\n[dim]from {len(pods)} pods[/dim]",
        title="📜 [green]Tailing Logs[/green]",
        subtitle="[yellow]Press Ctrl+C to stop[/yellow]",
        border_style="green",
    )
    console.print(panel)
    console.print()

    def signal_handler(sig, frame):
        console.print("\n[yellow]Stopping log tail...[/yellow]")
        sys.exit(0)

    signal.signal(signal.SIGINT, signal_handler)

    with ThreadPoolExecutor(max_workers=len(pods)) as executor:
        for i, pod in enumerate(pods):
            color = POD_COLORS[i % len(POD_COLORS)]
            executor.submit(tail_pod_log, pod, namespace, log_file, color)


@app.command()
def main(
    env: Annotated[Optional[Environment], typer.Option("--env", "-e", help="Environment (auto-detects if omitted)")] = None,
    pod_type: Annotated[str, typer.Option("--type", "-t", help="Pod name pattern to filter")] = "web",
    pod_num: Annotated[Optional[int], typer.Option("--pod", "-p", help="Pod number to connect to")] = None,
    namespace: Annotated[str, typer.Option("--namespace", "-n", help="Kubernetes namespace")] = "cms",
    log_file: Annotated[Optional[str], typer.Option("--log", "-l", help="Tail log file from all pods (default: /app/log/<env>.log)")] = None,
):
    """
    🚀 [bold]EKS Shell[/bold] - Connect to Kubernetes pods easily

    [dim]Examples:[/dim]
        eks-shell                              [dim]# List web pods[/dim]
        eks-shell --pod 1                      [dim]# Connect to pod #1[/dim]
        eks-shell -e prod -t api               [dim]# List api pods on prod[/dim]
        eks-shell --log                        [dim]# Tail default log[/dim]
        eks-shell -l /app/log/sidekiq.log      [dim]# Tail custom log[/dim]
    """
    # Detect environment if not specified
    if not env:
        detected = detect_env()
        if not detected:
            console.print("[yellow]⚠️  No --env specified and couldn't detect from current context[/yellow]")
            raise typer.BadParameter("Please specify --env (prod, dev, or stg)")
        env = Environment(detected)
        console.print(f"[blue]ℹ️  Detected environment:[/blue] [bold]{env.value}[/bold] [dim](from current context)[/dim]")

    config = ENV_CONFIG[env.value]
    cluster = config["cluster"]
    env_emoji = config["emoji"]
    env_long = config["long"]

    # Resolve log file path - use default if --log was passed without value
    if log_file == "":
        log_file = f"/app/log/{env_long}.log"

    # Check AWS session
    console.print("[blue]ℹ️  Checking AWS SSO session...[/blue]")
    if not check_aws_session():
        console.print("[yellow]⚠️  SSO session expired. Logging in...[/yellow]")
        aws_sso_login()
    console.print("[green]✅ AWS session active[/green]")

    # Update kubeconfig
    console.print(f"[blue]ℹ️  Updating kubeconfig for[/blue] [bold]{cluster}[/bold][blue]...[/blue]")
    update_kubeconfig(cluster)
    console.print(f"[green]✅ Connected to[/green] [bold]{cluster}[/bold]")

    # Get pods
    console.print(f"[blue]ℹ️  Fetching pods matching[/blue] [bold]{pod_type}[/bold] [blue]in namespace[/blue] [bold]{namespace}[/bold][blue]...[/blue]")
    pods = get_pods(namespace, pod_type)

    if not pods:
        console.print(f"[red]❌ No pods found matching '{pod_type}'[/red]")
        raise typer.Exit(1)

    display_pods(pods, env.value, env_emoji)

    # Log mode - tail from all pods
    if log_file is not None:
        # If empty string, use default
        if log_file == "":
            log_file = f"/app/log/{env_long}.log"
        tail_logs(pods, namespace, log_file)
        return

    # No pod specified - show hint and exit
    if pod_num is None:
        console.print(
            Panel(
                "[bold]--pod <number>[/bold]  Connect to a specific pod\n[bold]--log[/bold]           Tail logs from all pods",
                title="👆 [yellow]Next steps[/yellow]",
                border_style="yellow",
            )
        )
        return

    # Validate pod number
    if pod_num < 1 or pod_num > len(pods):
        console.print(f"[red]❌ Invalid pod number '{pod_num}'. Choose 1-{len(pods)}[/red]")
        raise typer.Exit(1)

    # Connect to pod
    pod = pods[pod_num - 1]
    exec_into_pod(pod, namespace, env.value, pod_type, pod_num, env_emoji)


if __name__ == "__main__":
    app()
