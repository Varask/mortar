# Serveur de prévisualisation UI sans backend Rust.
# Sert src/web en statique et stubbe l'API avec des données d'exemple.
# Usage : powershell -ExecutionPolicy Bypass -File tools\dev-preview.ps1 [-Port 3000]
param([int]$Port = 3000)

$web = Join-Path $PSScriptRoot "..\src\web" | Resolve-Path

$mortarsJson = '{"positions":[{"name":"M1","elevation":105,"x":0,"y":0,"ammo_type":"HE"},{"name":"M2","elevation":98,"x":-220,"y":150,"ammo_type":"SMOKE"}]}'
$targetsJson = '{"positions":[{"name":"T1","elevation":50,"x":500,"y":300,"target_type":"INFANTERIE","ammo_type":"HE"},{"name":"T2","elevation":80,"x":640,"y":-180,"target_type":"VEHICULE","ammo_type":"HE"}]}'
$calcJson = @'
{"distance_m":583.1,"azimuth_deg":59.0,"azimuth_mils":1048.9,"elevation_diff_m":55.0,
"mortar_ammo":"HE","target_type":"INFANTERIE","recommended_ammo":"HE",
"selected_solution":{"ammo_type":"HE",
 "elevations":{"0R":1150.2,"1R":1128.5,"2R":1106.8,"3R":1085.1,"4R":1063.4},
 "dispersions":{"0R":35.0,"1R":80.5,"2R":136.5,"3R":189.0,"4R":241.5}},
"solutions":{
 "PRACTICE":{"0R":1140.0,"1R":1120.0,"2R":1100.0,"3R":1080.0,"4R":1060.0},
 "HE":{"0R":1150.2,"1R":1128.5,"2R":1106.8,"3R":1085.1,"4R":1063.4},
 "SMOKE":{"0R":null,"1R":1130.0,"2R":1110.0,"3R":1090.0,"4R":1070.0},
 "FLARE":{"0R":null,"1R":1135.0,"2R":1115.0,"3R":1095.0,"4R":1075.0}},
"dispersions":{
 "PRACTICE":{"0R":30.0,"1R":75.0,"2R":130.0,"3R":182.0,"4R":230.0},
 "HE":{"0R":35.0,"1R":80.5,"2R":136.5,"3R":189.0,"4R":241.5},
 "SMOKE":{"0R":null,"1R":82.0,"2R":140.0,"3R":195.0,"4R":250.0},
 "FLARE":{"0R":null,"1R":85.0,"2R":145.0,"3R":200.0,"4R":255.0}}}
'@

$mime = @{ ".html"="text/html; charset=utf-8"; ".css"="text/css; charset=utf-8"; ".js"="application/javascript; charset=utf-8"; ".svg"="image/svg+xml" }

$listener = New-Object System.Net.HttpListener
$listener.Prefixes.Add("http://localhost:$Port/")
$listener.Start()
Write-Host "Preview server on http://localhost:$Port (web: $web)"

try {
    while ($listener.IsListening) {
        $ctx = $listener.GetContext()
        $req = $ctx.Request
        $res = $ctx.Response
        $path = $req.Url.AbsolutePath

        $body = $null
        if ($path -like "/api/*") {
            $res.ContentType = "application/json; charset=utf-8"
            switch -Wildcard ($path) {
                "/api/health"    { $body = '{"status":"ok"}' }
                "/api/mortars"   { $body = $mortarsJson }
                "/api/targets"   { $body = $targetsJson }
                "/api/calculate" { $body = $calcJson }
                default          { $body = '{"status":"ok"}' }
            }
        }
        else {
            if ($path -eq "/") { $path = "/index.html" }
            $file = Join-Path $web ($path.TrimStart('/'))
            if (Test-Path $file) {
                $ext = [IO.Path]::GetExtension($file)
                $res.ContentType = if ($mime[$ext]) { $mime[$ext] } else { "application/octet-stream" }
                $bytes = [IO.File]::ReadAllBytes($file)
                $res.OutputStream.Write($bytes, 0, $bytes.Length)
                $res.Close()
                continue
            }
            $res.StatusCode = 404
            $body = "404"
            $res.ContentType = "text/plain"
        }

        $bytes = [Text.Encoding]::UTF8.GetBytes($body)
        $res.OutputStream.Write($bytes, 0, $bytes.Length)
        $res.Close()
    }
}
finally {
    $listener.Stop()
}
