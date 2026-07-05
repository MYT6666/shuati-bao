$ErrorActionPreference = 'Stop'
$baseDir = 'D:\桌面\刷题宝'

Get-ChildItem -Path $baseDir -Filter 'wechat-*.png' -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue

$items = @(
    @{ n = 'wechat-cover.png'; p = 'minimalist tech product banner blue gradient white quiz card icon centered clean modern' },
    @{ n = 'wechat-1-practice.png'; p = 'modern desktop quiz app interface question 4 multiple choice options blue accent dark sidebar' },
    @{ n = 'wechat-2-stats.png'; p = 'modern data dashboard 365 day contribution heatmap green colors statistics cards dark sidebar' },
    @{ n = 'wechat-3-import.png'; p = 'modern file upload interface large dashed border drop zone cloud upload icon dark sidebar' }
)

foreach ($it in $items) {
    $url = "https://trae-api-cn.mchost.guru/api/ide/v1/text_to_image?prompt=$([uri]::EscapeDataString($it.p))&image_size=landscape_16_9"
    $out = Join-Path $baseDir $it.n
    Write-Host "Generating $($it.n)..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing -TimeoutSec 30
        Write-Host "  OK: $((Get-Item $out).Length) bytes"
    } catch {
        Write-Host "  FAIL: $($_.Exception.Message)"
    }
    Start-Sleep -Seconds 5
}
