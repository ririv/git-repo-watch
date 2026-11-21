# 只在临时 Windows runner 中注册任务和操作 Git 测试仓库。
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
if (!$IsWindows -or $env:GITHUB_ACTIONS -ne 'true') {
    throw '此测试只能在 Windows CI 中运行。'
}

$identity = ConvertFrom-StringData (Get-Content "$PSScriptRoot/../../application.env" -Raw)
$gw = "$PSScriptRoot/../../target/release/app.exe"
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$label = "$($identity.APP_SERVICE_LABEL)-$sid"
$profile = Join-Path $env:RUNNER_TEMP "仓库检查 $([guid]::NewGuid())"
$repo = Join-Path $profile 'repository'
$remote = Join-Path $profile 'remote.git'
New-Item -ItemType Directory -Path $profile | Out-Null
$installed = $false

try {
    & $gw --home $profile init
    git init --bare --initial-branch=main $remote
    git init --initial-branch=main $repo
    git -C $repo -c user.name=CI -c user.email=ci@example.invalid commit --allow-empty -m initial
    git -C $repo remote add origin $remote
    git -C $repo push --set-upstream origin HEAD
    & $gw --home $profile add $repo
    & $gw --home $profile install
    $installed = $true

    $task = Get-ScheduledTask -TaskName $label -TaskPath '\'
    if (!$task.Settings.Enabled -or @($task.Actions).Count -ne 1 -or
        $task.Actions.Arguments -ne "--home `"$profile`" scheduled-tick") {
        throw '任务计划启动参数不正确。'
    }

    $status = & $gw --home $profile status --json | Out-String | ConvertFrom-Json
    if ($status.registration -ne 'loaded' -or $status.schedule.status -ne 'anchored' -or
        $status.report.status -ne 'missing') {
        throw '启用后的任务状态或报告状态不正确。'
    }

    & $gw --home $profile schedule disable
    & $gw --home $profile interval 12h
    & $gw --home $profile schedule apply
    if ((Get-ScheduledTask -TaskName $label -TaskPath '\').Settings.Enabled) {
        throw '更新间隔重新启用了停用的任务。'
    }
    $status = & $gw --home $profile status --json | Out-String | ConvertFrom-Json
    if ($status.registration -ne 'not_loaded' -or $status.schedule.plan.interval_seconds -ne 43200) {
        throw '更新后的停用状态或间隔不正确。'
    }

    & $gw --home $profile schedule enable
    if (!(Get-ScheduledTask -TaskName $label -TaskPath '\').Settings.Enabled) {
        throw '任务计划未启用。'
    }
    & $gw --home $profile scheduled-check
    & $gw --home $profile report --json | Out-String | ConvertFrom-Json | Out-Null
    & $gw --home $profile uninstall
    $installed = $false
    if (Get-ScheduledTask -TaskName $label -TaskPath '\' -ErrorAction SilentlyContinue) {
        throw '卸载没有移除任务。'
    }
    if (!(Test-Path (Join-Path $profile 'config.toml'))) {
        throw '卸载移除了配置。'
    }
} finally {
    if ($installed) { & $gw --home $profile uninstall }
    Remove-Item $profile -Recurse -Force
}
