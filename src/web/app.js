// ============================================================
// MORTAR — Fire Direction Center
// État global
// ============================================================
let mortars = [];
let targets = [];
let selectedMortar = null;
let selectedTarget = null;
let lastSolution = null;      // dernière solution de tir reçue
let activeRing = null;        // anneau sélectionné pour la dispersion sur carte
let mapAnimT = 0;             // phase d'animation de la ligne de tir

const RINGS = ['0R', '1R', '2R', '3R', '4R'];
const AMMO_TYPES = ['PRACTICE', 'HE', 'SMOKE', 'FLARE'];
const REDUCED_MOTION = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

document.addEventListener('DOMContentLoaded', async () => {
    await Promise.all([loadMortars(), loadTargets()]);

    // Mode démo (#demo) : sélectionne la première paire et calcule
    if (location.hash === '#demo' && mortars.length && targets.length) {
        selectedMortar = mortars[0].name;
        selectedTarget = targets[0].name;
        document.getElementById('selected-mortar').value = selectedMortar;
        document.getElementById('selected-target').value = selectedTarget;
        renderMortarsList();
        renderTargetsList();
        calculate();
    }
    initMap();
    initCompassTicks();
    startZuluClock();
    pingHealth();
    setInterval(pingHealth, 15000);

    document.getElementById('add-mortar-btn').addEventListener('click', addMortar);
    document.getElementById('add-target-btn').addEventListener('click', addTarget);
    document.getElementById('calculate-btn').addEventListener('click', calculate);
    document.getElementById('apply-correction-btn').addEventListener('click', applyCorrection);

    document.getElementById('selected-mortar').addEventListener('change', (e) => {
        selectedMortar = e.target.value || null;
        renderMortarsList();
        requestMapDraw();
    });

    document.getElementById('selected-target').addEventListener('change', (e) => {
        selectedTarget = e.target.value || null;
        renderTargetsList();
        requestMapDraw();
    });

    document.querySelectorAll('#mortar-name, #mortar-elevation, #mortar-x, #mortar-y').forEach(input => {
        input.addEventListener('keypress', (e) => { if (e.key === 'Enter') addMortar(); });
    });

    document.querySelectorAll('#target-name, #target-elevation, #target-x, #target-y').forEach(input => {
        input.addEventListener('keypress', (e) => { if (e.key === 'Enter') addTarget(); });
    });
});

// ============================================================
// Horloge Zulu + santé serveur
// ============================================================
function startZuluClock() {
    const el = document.getElementById('zulu-clock');
    const tick = () => {
        const d = new Date();
        const p = (n) => String(n).padStart(2, '0');
        el.textContent = `${p(d.getUTCHours())}:${p(d.getUTCMinutes())}:${p(d.getUTCSeconds())}Z`;
    };
    tick();
    setInterval(tick, 1000);
}

async function pingHealth() {
    const dot = document.getElementById('status-dot');
    const label = document.getElementById('status-label');
    try {
        const r = await fetch('/api/health');
        if (r.ok) {
            dot.className = 'status-dot online';
            label.textContent = 'EN LIGNE';
            return;
        }
        throw new Error();
    } catch {
        dot.className = 'status-dot offline';
        label.textContent = 'HORS LIGNE';
    }
}

// ============================================================
// Appels API
// ============================================================
async function loadMortars() {
    try {
        const response = await fetch('/api/mortars');
        const data = await response.json();
        mortars = data.positions || [];
        renderMortarsList();
        updateMortarsDropdown();
        requestMapDraw();
    } catch (error) {
        console.error('Failed to load mortars:', error);
    }
}

async function loadTargets() {
    try {
        const response = await fetch('/api/targets');
        const data = await response.json();
        targets = data.positions || [];
        renderTargetsList();
        updateTargetsDropdown();
        requestMapDraw();
    } catch (error) {
        console.error('Failed to load targets:', error);
    }
}

async function addMortar() {
    const name = document.getElementById('mortar-name').value.trim();
    const elevation = parseFloat(document.getElementById('mortar-elevation').value) || 0;
    const x = parseFloat(document.getElementById('mortar-x').value) || 0;
    const y = parseFloat(document.getElementById('mortar-y').value) || 0;

    if (!name) {
        showToast('Le nom du mortier est requis', 'error');
        return;
    }

    try {
        const response = await fetch('/api/mortars', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, elevation, x, y })
        });

        const data = await response.json();

        if (response.ok) {
            showToast(`Mortier '${name}' déployé`, 'success');
            document.getElementById('mortar-name').value = '';
            loadMortars();
        } else {
            showToast(data.error || 'Erreur', 'error');
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function deleteMortar(name) {
    try {
        const response = await fetch('/api/mortars', {
            method: 'DELETE',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name })
        });

        if (response.ok) {
            showToast(`Mortier '${name}' retiré`, 'success');
            if (selectedMortar === name) {
                selectedMortar = null;
                document.getElementById('selected-mortar').value = '';
            }
            loadMortars();
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function addTarget() {
    const name = document.getElementById('target-name').value.trim();
    const elevation = parseFloat(document.getElementById('target-elevation').value) || 0;
    const x = parseFloat(document.getElementById('target-x').value) || 0;
    const y = parseFloat(document.getElementById('target-y').value) || 0;
    const target_type = document.getElementById('target-type').value;
    const ammo_type = document.getElementById('target-ammo').value;

    if (!name) {
        showToast('Le nom de la cible est requis', 'error');
        return;
    }

    try {
        const response = await fetch('/api/targets', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, elevation, x, y, target_type, ammo_type })
        });

        const data = await response.json();

        if (response.ok) {
            showToast(`Cible '${name}' désignée [${target_type}] [${ammo_type}]`, 'success');
            document.getElementById('target-name').value = '';
            loadTargets();
        } else {
            showToast(data.error || 'Erreur', 'error');
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function deleteTarget(name) {
    try {
        const response = await fetch('/api/targets', {
            method: 'DELETE',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name })
        });

        if (response.ok) {
            showToast(`Cible '${name}' retirée`, 'success');
            if (selectedTarget === name) {
                selectedTarget = null;
                document.getElementById('selected-target').value = '';
            }
            loadTargets();
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function updateTargetType(name, target_type) {
    try {
        const response = await fetch('/api/targets/type', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, target_type })
        });

        if (response.ok) {
            showToast(`Type ${name} → ${target_type}`, 'success');
            loadTargets();
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function updateTargetAmmo(name, ammo_type) {
    try {
        const response = await fetch('/api/targets/ammo', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, ammo_type })
        });

        if (response.ok) {
            showToast(`Ogive ${name} → ${ammo_type}`, 'success');
            loadTargets();
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function applyCorrection() {
    if (!selectedTarget) {
        showToast('Sélectionnez une cible d\'abord', 'error');
        return;
    }

    const vertical_m = parseFloat(document.getElementById('correction-vertical').value) || 0;
    const horizontal_m = parseFloat(document.getElementById('correction-horizontal').value) || 0;

    if (vertical_m === 0 && horizontal_m === 0) {
        showToast('Entrez une déviation à corriger', 'error');
        return;
    }

    try {
        const response = await fetch('/api/targets/correct', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ target_name: selectedTarget, vertical_m, horizontal_m })
        });

        const data = await response.json();

        if (response.ok) {
            showToast(`Correction appliquée : ${data.corrected}`, 'success');
            document.getElementById('correction-vertical').value = '0';
            document.getElementById('correction-horizontal').value = '0';

            await loadTargets();
            selectedTarget = data.corrected;
            document.getElementById('selected-target').value = data.corrected;
            renderTargetsList();
            requestMapDraw();

            if (selectedMortar) calculate();
        } else {
            showToast(data.error || 'Erreur de correction', 'error');
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function calculate() {
    const btn = document.getElementById('calculate-btn');
    const resultsSection = document.getElementById('results');
    const errorSection = document.getElementById('error');

    errorSection.classList.add('hidden');

    if (!selectedMortar || !selectedTarget) {
        showError('Sélectionnez un mortier et une cible');
        return;
    }

    btn.classList.add('loading');
    btn.disabled = true;

    try {
        const response = await fetch('/api/calculate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ mortar_name: selectedMortar, target_name: selectedTarget })
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.error || 'Erreur de calcul');
        }

        displayResults(data, selectedMortar, selectedTarget);
    } catch (error) {
        resultsSection.classList.add('hidden');
        lastSolution = null;
        requestMapDraw();
        showError(error.message || 'Erreur de connexion au serveur');
    } finally {
        btn.classList.remove('loading');
        btn.disabled = false;
    }
}

// ============================================================
// Rendu des listes
// ============================================================
function renderMortarsList() {
    const list = document.getElementById('mortars-list');
    document.getElementById('mortars-count').textContent = mortars.length;
    list.innerHTML = '';

    if (mortars.length === 0) {
        list.innerHTML = '<li class="empty-message">Aucun mortier déployé</li>';
        return;
    }

    for (const mortar of mortars) {
        const li = document.createElement('li');
        li.className = selectedMortar === mortar.name ? 'selected' : '';
        li.innerHTML = `
            <div class="position-info">
                <span class="position-name">${mortar.name}</span>
                <span class="position-coords">X:${mortar.x} Y:${mortar.y} E:${mortar.elevation}m</span>
            </div>
            <div class="item-actions">
                <button class="btn-delete" aria-label="Supprimer ${mortar.name}">✕</button>
            </div>
        `;

        li.querySelector('.position-info').addEventListener('click', () => {
            selectedMortar = mortar.name;
            document.getElementById('selected-mortar').value = mortar.name;
            renderMortarsList();
            requestMapDraw();
        });

        li.querySelector('.btn-delete').addEventListener('click', (e) => {
            e.stopPropagation();
            deleteMortar(mortar.name);
        });

        list.appendChild(li);
    }
}

function renderTargetsList() {
    const list = document.getElementById('targets-list');
    document.getElementById('targets-count').textContent = targets.length;
    list.innerHTML = '';

    if (targets.length === 0) {
        list.innerHTML = '<li class="empty-message">Aucune cible désignée</li>';
        return;
    }

    for (const target of targets) {
        const li = document.createElement('li');
        li.className = selectedTarget === target.name ? 'selected' : '';
        li.innerHTML = `
            <div class="position-info">
                <span class="position-name">${target.name}</span>
                <span class="position-coords">X:${target.x} Y:${target.y} E:${target.elevation}m</span>
            </div>
            <div class="item-actions">
                <select class="type-select" aria-label="Type de ${target.name}">
                    <option value="INFANTERIE" ${target.target_type === 'INFANTERIE' ? 'selected' : ''}>INF</option>
                    <option value="VEHICULE" ${target.target_type === 'VEHICULE' ? 'selected' : ''}>VEH</option>
                    <option value="SOUTIEN" ${target.target_type === 'SOUTIEN' ? 'selected' : ''}>SOU</option>
                </select>
                <select class="ammo-select" aria-label="Ogive de ${target.name}">
                    <option value="HE" ${target.ammo_type === 'HE' ? 'selected' : ''}>HE</option>
                    <option value="PRACTICE" ${target.ammo_type === 'PRACTICE' ? 'selected' : ''}>PRAC</option>
                    <option value="SMOKE" ${target.ammo_type === 'SMOKE' ? 'selected' : ''}>SMK</option>
                    <option value="FLARE" ${target.ammo_type === 'FLARE' ? 'selected' : ''}>FLR</option>
                </select>
                <button class="btn-delete" aria-label="Supprimer ${target.name}">✕</button>
            </div>
        `;

        li.querySelector('.position-info').addEventListener('click', () => {
            selectedTarget = target.name;
            document.getElementById('selected-target').value = target.name;
            renderTargetsList();
            requestMapDraw();
        });

        li.querySelector('.type-select').addEventListener('change', (e) => {
            e.stopPropagation();
            updateTargetType(target.name, e.target.value);
        });

        li.querySelector('.ammo-select').addEventListener('change', (e) => {
            e.stopPropagation();
            updateTargetAmmo(target.name, e.target.value);
        });

        li.querySelector('.btn-delete').addEventListener('click', (e) => {
            e.stopPropagation();
            deleteTarget(target.name);
        });

        list.appendChild(li);
    }
}

function updateMortarsDropdown() {
    const select = document.getElementById('selected-mortar');
    const currentValue = select.value;

    select.innerHTML = '<option value="">— Sélectionner —</option>';
    for (const mortar of mortars) {
        const option = document.createElement('option');
        option.value = mortar.name;
        option.textContent = mortar.name;
        select.appendChild(option);
    }

    if (currentValue && mortars.some(m => m.name === currentValue)) {
        select.value = currentValue;
    } else {
        selectedMortar = null;
    }
}

function updateTargetsDropdown() {
    const select = document.getElementById('selected-target');
    const currentValue = select.value;

    select.innerHTML = '<option value="">— Sélectionner —</option>';
    for (const target of targets) {
        const option = document.createElement('option');
        option.value = target.name;
        option.textContent = `${target.name} [${target.target_type}] [${target.ammo_type}]`;
        select.appendChild(option);
    }

    if (currentValue && targets.some(t => t.name === currentValue)) {
        select.value = currentValue;
    } else {
        selectedTarget = null;
    }
}

// ============================================================
// Affichage de la solution
// ============================================================
function displayResults(data, mortarName, targetName) {
    const resultsSection = document.getElementById('results');
    lastSolution = { ...data, mortarName, targetName };

    document.getElementById('solution-mortar').textContent = mortarName;
    document.getElementById('solution-target').textContent = targetName;

    animateValue(document.getElementById('distance'), data.distance_m, 1);
    document.getElementById('azimuth').textContent =
        data.azimuth_deg.toFixed(1) + ' / ' + data.azimuth_mils.toFixed(0);
    animateValue(document.getElementById('elevation-diff'), data.elevation_diff_m, 1);

    setCompass(data.azimuth_deg);

    document.getElementById('mortar-ammo-display').textContent = data.mortar_ammo;
    document.getElementById('target-type-display').textContent = data.target_type;
    document.getElementById('recommended-ammo').textContent = '★ ' + data.recommended_ammo;

    // Cartes d'élévation (cliquables → dispersion sur la carte)
    if (data.selected_solution) {
        document.getElementById('selected-ammo-type').textContent = data.selected_solution.ammo_type;
        const cardsContainer = document.getElementById('elevation-cards');
        cardsContainer.innerHTML = '';

        // anneau actif par défaut : premier disponible
        if (!activeRing || data.selected_solution.elevations[activeRing] == null) {
            activeRing = RINGS.find(r => data.selected_solution.elevations[r] != null) || null;
        }

        for (const ring of RINGS) {
            const elev = data.selected_solution.elevations[ring];
            const disp = data.selected_solution.dispersions ? data.selected_solution.dispersions[ring] : null;
            const card = document.createElement('div');
            card.className = 'elevation-card' + (ring === activeRing && elev != null ? ' active' : '');
            card.innerHTML = `
                <div class="ring">${ring}</div>
                <div class="value ${elev == null ? 'na' : ''}">${elev != null ? elev.toFixed(1) : 'N/A'}</div>
                <div class="dispersion ${disp == null ? 'na' : ''}">±${disp != null ? disp.toFixed(1) : '--'}m</div>
            `;
            if (elev != null) {
                card.addEventListener('click', () => {
                    activeRing = ring;
                    cardsContainer.querySelectorAll('.elevation-card').forEach(c => c.classList.remove('active'));
                    card.classList.add('active');
                    requestMapDraw();
                });
            }
            cardsContainer.appendChild(card);
        }
    }

    // Table complète
    const tbody = document.getElementById('solutions-body');
    tbody.innerHTML = '';

    for (const ammoType of AMMO_TYPES) {
        const row = document.createElement('tr');
        if (ammoType === data.mortar_ammo) row.classList.add('highlighted-row');

        const typeCell = document.createElement('td');
        typeCell.textContent = ammoType;
        row.appendChild(typeCell);

        const ammoSolutions = data.solutions[ammoType] || {};
        const ammoDispersions = data.dispersions ? (data.dispersions[ammoType] || {}) : {};
        for (const ring of RINGS) {
            const cell = document.createElement('td');
            const elev = ammoSolutions[ring];
            const disp = ammoDispersions[ring];

            if (elev !== null && elev !== undefined) {
                let content = elev.toFixed(1);
                if (disp !== null && disp !== undefined) {
                    content += `<span class="table-disp">±${disp.toFixed(1)}</span>`;
                }
                cell.innerHTML = content;
            } else {
                cell.textContent = 'N/A';
                cell.classList.add('na');
            }

            row.appendChild(cell);
        }

        tbody.appendChild(row);
    }

    resultsSection.classList.remove('hidden');
    requestMapDraw();
}

// Compteur animé pour les métriques
function animateValue(el, value, decimals) {
    if (REDUCED_MOTION) {
        el.textContent = value.toFixed(decimals);
        return;
    }
    const duration = 600;
    const start = performance.now();
    const from = parseFloat(el.textContent) || 0;
    const step = (now) => {
        const t = Math.min(1, (now - start) / duration);
        const eased = 1 - Math.pow(1 - t, 3);
        el.textContent = (from + (value - from) * eased).toFixed(decimals);
        if (t < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
}

// ============================================================
// Compas SVG
// ============================================================
function initCompassTicks() {
    const g = document.getElementById('compass-ticks');
    if (!g) return;
    for (let deg = 0; deg < 360; deg += 15) {
        const major = deg % 45 === 0;
        const rad = (deg * Math.PI) / 180;
        const r1 = major ? 46 : 50;
        const r2 = 54;
        const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        line.setAttribute('x1', 60 + r1 * Math.sin(rad));
        line.setAttribute('y1', 60 - r1 * Math.cos(rad));
        line.setAttribute('x2', 60 + r2 * Math.sin(rad));
        line.setAttribute('y2', 60 - r2 * Math.cos(rad));
        g.appendChild(line);
    }
}

function setCompass(deg) {
    const needle = document.getElementById('compass-needle');
    if (needle) needle.style.transform = `rotate(${deg}deg)`;
}

// ============================================================
// Carte tactique (canvas)
// Convention : +X = Est, +Y = Nord (azimut depuis le Nord)
// ============================================================
const map = {
    canvas: null,
    ctx: null,
    dpr: 1,
    // transformation monde → écran
    scale: 1,
    offsetX: 0,
    offsetY: 0,
};

function initMap() {
    map.canvas = document.getElementById('tactical-map');
    map.ctx = map.canvas.getContext('2d');

    const resize = () => {
        const rect = map.canvas.parentElement.getBoundingClientRect();
        map.dpr = window.devicePixelRatio || 1;
        map.canvas.width = Math.max(1, Math.round(rect.width * map.dpr));
        map.canvas.height = Math.max(1, Math.round(rect.height * map.dpr));
        requestMapDraw();
    };

    new ResizeObserver(resize).observe(map.canvas.parentElement);
    resize();

    // Clic : sélectionne l'unité la plus proche
    map.canvas.addEventListener('click', (e) => {
        const world = screenToWorld(e);
        if (!world) return;
        const hit = nearestUnit(world.x, world.y);
        if (!hit) return;

        if (hit.kind === 'mortar') {
            selectedMortar = hit.unit.name;
            document.getElementById('selected-mortar').value = hit.unit.name;
            renderMortarsList();
        } else {
            selectedTarget = hit.unit.name;
            document.getElementById('selected-target').value = hit.unit.name;
            renderTargetsList();
        }
        requestMapDraw();
    });

    // Double-clic : pré-remplit X/Y du formulaire cible
    map.canvas.addEventListener('dblclick', (e) => {
        const world = screenToWorld(e);
        if (!world) return;
        document.getElementById('target-x').value = Math.round(world.x);
        document.getElementById('target-y').value = Math.round(world.y);
        showToast(`Coordonnées copiées : X=${Math.round(world.x)} Y=${Math.round(world.y)}`, 'success');
    });

    if (!REDUCED_MOTION) {
        const loop = () => {
            mapAnimT = (mapAnimT + 0.5) % 24;
            if (lastSolution && selectedMortar && selectedTarget) drawMap();
            requestAnimationFrame(loop);
        };
        requestAnimationFrame(loop);
    }
}

let mapDrawQueued = false;
function requestMapDraw() {
    if (mapDrawQueued) return;
    mapDrawQueued = true;
    requestAnimationFrame(() => {
        mapDrawQueued = false;
        drawMap();
    });
}

function computeTransform(w, h) {
    const units = [...mortars, ...targets];
    let minX = -50, maxX = 50, minY = -50, maxY = 50;
    if (units.length > 0) {
        minX = Math.min(...units.map(u => u.x));
        maxX = Math.max(...units.map(u => u.x));
        minY = Math.min(...units.map(u => u.y));
        maxY = Math.max(...units.map(u => u.y));
    }
    const spanX = Math.max(maxX - minX, 100);
    const spanY = Math.max(maxY - minY, 100);
    const pad = 0.18;
    map.scale = Math.min(w / (spanX * (1 + pad * 2)), h / (spanY * (1 + pad * 2)));
    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    map.offsetX = w / 2 - cx * map.scale;
    map.offsetY = h / 2 + cy * map.scale; // +Y monde = Nord = haut écran
}

function worldToScreen(x, y) {
    return { x: map.offsetX + x * map.scale, y: map.offsetY - y * map.scale };
}

function screenToWorld(e) {
    const rect = map.canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    if (!map.scale) return null;
    return screenToWorldRaw(sx, sy);
}

function screenToWorldRaw(sx, sy) {
    return {
        x: (sx - map.offsetX) / map.scale,
        y: (map.offsetY - sy) / map.scale,
    };
}

function nearestUnit(wx, wy) {
    let best = null;
    let bestDist = 25 / map.scale; // rayon de capture ~25px
    for (const m of mortars) {
        const d = Math.hypot(m.x - wx, m.y - wy);
        if (d < bestDist) { bestDist = d; best = { kind: 'mortar', unit: m }; }
    }
    for (const t of targets) {
        const d = Math.hypot(t.x - wx, t.y - wy);
        if (d < bestDist) { bestDist = d; best = { kind: 'target', unit: t }; }
    }
    return best;
}

function niceStep(raw) {
    const pow = Math.pow(10, Math.floor(Math.log10(raw)));
    const n = raw / pow;
    if (n < 1.5) return pow;
    if (n < 3.5) return 2 * pow;
    if (n < 7.5) return 5 * pow;
    return 10 * pow;
}

function drawMap() {
    const ctx = map.ctx;
    if (!ctx) return;
    const w = map.canvas.width / map.dpr;
    const h = map.canvas.height / map.dpr;

    ctx.setTransform(map.dpr, 0, 0, map.dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    computeTransform(w, h);

    const css = getComputedStyle(document.documentElement);
    const C = {
        grid: 'rgba(92, 255, 143, 0.07)',
        gridMajor: 'rgba(92, 255, 143, 0.14)',
        axis: 'rgba(92, 255, 143, 0.25)',
        label: css.getPropertyValue('--text-faint').trim() || '#4a5c50',
        mortar: css.getPropertyValue('--phosphor').trim() || '#5cff8f',
        target: css.getPropertyValue('--danger').trim() || '#ff5c5c',
        amber: css.getPropertyValue('--amber').trim() || '#ffb454',
        dim: css.getPropertyValue('--text-dim').trim() || '#7e9486',
    };

    // --- Grille ---
    const targetPx = 70; // espacement souhaité en px
    const step = niceStep(targetPx / map.scale);
    const topLeft = screenToWorldRaw(0, 0);
    const bottomRight = screenToWorldRaw(w, h);

    ctx.font = '10px "Share Tech Mono", monospace';
    ctx.fillStyle = C.label;

    const x0 = Math.floor(topLeft.x / step) * step;
    for (let x = x0; x <= bottomRight.x; x += step) {
        const s = worldToScreen(x, 0);
        const major = Math.round(x / step) % 5 === 0;
        ctx.strokeStyle = x === 0 ? C.axis : (major ? C.gridMajor : C.grid);
        ctx.beginPath();
        ctx.moveTo(s.x, 0);
        ctx.lineTo(s.x, h);
        ctx.stroke();
        if (major || x === 0) ctx.fillText(String(Math.round(x)), s.x + 3, h - 6);
    }

    const y0 = Math.floor(bottomRight.y / step) * step;
    for (let y = y0; y <= topLeft.y; y += step) {
        const s = worldToScreen(0, y);
        const major = Math.round(y / step) % 5 === 0;
        ctx.strokeStyle = y === 0 ? C.axis : (major ? C.gridMajor : C.grid);
        ctx.beginPath();
        ctx.moveTo(0, s.y);
        ctx.lineTo(w, s.y);
        ctx.stroke();
        if (major || y === 0) ctx.fillText(String(Math.round(y)), 5, s.y - 3);
    }

    // Échelle affichée
    document.getElementById('map-scale').textContent = `GRILLE ${step} m`;

    // Indicateur Nord
    ctx.fillStyle = C.dim;
    ctx.font = '12px "Share Tech Mono", monospace';
    ctx.fillText('N ↑', 12, 20);

    // --- État vide ---
    if (mortars.length === 0 && targets.length === 0) {
        ctx.fillStyle = C.label;
        ctx.font = '13px "Share Tech Mono", monospace';
        ctx.textAlign = 'center';
        ctx.fillText('AUCUNE UNITÉ DÉPLOYÉE', w / 2, h / 2);
        ctx.textAlign = 'left';
        return;
    }

    // --- Ligne de tir (sélection courante) ---
    const m = mortars.find(u => u.name === selectedMortar);
    const t = targets.find(u => u.name === selectedTarget);

    if (m && t && lastSolution &&
        lastSolution.mortarName === m.name && lastSolution.targetName === t.name) {
        const sm = worldToScreen(m.x, m.y);
        const st = worldToScreen(t.x, t.y);

        // Dispersion de l'anneau actif autour de la cible
        const sel = lastSolution.selected_solution;
        if (sel && sel.dispersions && activeRing && sel.dispersions[activeRing] != null) {
            const r = sel.dispersions[activeRing] * map.scale;
            ctx.beginPath();
            ctx.arc(st.x, st.y, r, 0, Math.PI * 2);
            ctx.fillStyle = 'rgba(255, 180, 84, 0.08)';
            ctx.fill();
            ctx.setLineDash([5, 4]);
            ctx.strokeStyle = C.amber;
            ctx.lineWidth = 1;
            ctx.stroke();
            ctx.setLineDash([]);
            ctx.fillStyle = C.amber;
            ctx.font = '10px "Share Tech Mono", monospace';
            ctx.fillText(`${activeRing} ±${sel.dispersions[activeRing].toFixed(0)}m`, st.x + r + 6, st.y);
        }

        // Trajectoire pointillée animée
        ctx.beginPath();
        ctx.moveTo(sm.x, sm.y);
        ctx.lineTo(st.x, st.y);
        ctx.setLineDash([10, 14]);
        ctx.lineDashOffset = -mapAnimT;
        ctx.strokeStyle = C.amber;
        ctx.lineWidth = 1.6;
        ctx.stroke();
        ctx.setLineDash([]);

        // Étiquette distance au milieu
        const mid = { x: (sm.x + st.x) / 2, y: (sm.y + st.y) / 2 };
        ctx.fillStyle = C.amber;
        ctx.font = '11px "Share Tech Mono", monospace';
        ctx.fillText(`${lastSolution.distance_m.toFixed(0)} m / ${lastSolution.azimuth_deg.toFixed(1)}°`, mid.x + 8, mid.y - 8);
    } else if (m && t) {
        // Pas encore calculé : ligne discrète
        const sm = worldToScreen(m.x, m.y);
        const st = worldToScreen(t.x, t.y);
        ctx.beginPath();
        ctx.moveTo(sm.x, sm.y);
        ctx.lineTo(st.x, st.y);
        ctx.setLineDash([3, 6]);
        ctx.strokeStyle = 'rgba(126, 148, 134, 0.4)';
        ctx.lineWidth = 1;
        ctx.stroke();
        ctx.setLineDash([]);
    }

    // --- Unités ---
    for (const u of mortars) drawMortar(ctx, u, C, u.name === selectedMortar);
    for (const u of targets) drawTarget(ctx, u, C, u.name === selectedTarget);
}

function drawMortar(ctx, u, C, selected) {
    const s = worldToScreen(u.x, u.y);
    const r = selected ? 9 : 7;

    if (selected) {
        ctx.beginPath();
        ctx.arc(s.x, s.y, r + 7, 0, Math.PI * 2);
        ctx.strokeStyle = 'rgba(92, 255, 143, 0.45)';
        ctx.lineWidth = 1;
        ctx.stroke();
    }

    // Triangle (symbole mortier)
    ctx.beginPath();
    ctx.moveTo(s.x, s.y - r);
    ctx.lineTo(s.x + r * 0.9, s.y + r * 0.75);
    ctx.lineTo(s.x - r * 0.9, s.y + r * 0.75);
    ctx.closePath();
    ctx.fillStyle = selected ? C.mortar : 'rgba(92, 255, 143, 0.75)';
    ctx.fill();

    ctx.fillStyle = C.mortar;
    ctx.font = (selected ? 'bold 12px' : '11px') + ' "Share Tech Mono", monospace';
    ctx.fillText(u.name, s.x + r + 5, s.y + 4);
}

function drawTarget(ctx, u, C, selected) {
    const s = worldToScreen(u.x, u.y);
    const r = selected ? 9 : 7;

    if (selected) {
        ctx.beginPath();
        ctx.arc(s.x, s.y, r + 7, 0, Math.PI * 2);
        ctx.strokeStyle = 'rgba(255, 92, 92, 0.45)';
        ctx.lineWidth = 1;
        ctx.stroke();
    }

    // Losange + croix (symbole cible)
    ctx.beginPath();
    ctx.moveTo(s.x, s.y - r);
    ctx.lineTo(s.x + r, s.y);
    ctx.lineTo(s.x, s.y + r);
    ctx.lineTo(s.x - r, s.y);
    ctx.closePath();
    ctx.strokeStyle = selected ? C.target : 'rgba(255, 92, 92, 0.8)';
    ctx.lineWidth = selected ? 2 : 1.4;
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(s.x - r * 0.45, s.y);
    ctx.lineTo(s.x + r * 0.45, s.y);
    ctx.moveTo(s.x, s.y - r * 0.45);
    ctx.lineTo(s.x, s.y + r * 0.45);
    ctx.stroke();

    ctx.fillStyle = C.target;
    ctx.font = (selected ? 'bold 12px' : '11px') + ' "Share Tech Mono", monospace';
    ctx.fillText(u.name, s.x + r + 5, s.y + 4);
}

// ============================================================
// Notifications
// ============================================================
function showError(message) {
    const errorSection = document.getElementById('error');
    errorSection.textContent = message;
    errorSection.classList.remove('hidden');
}

function showToast(message, type = 'success') {
    const container = document.getElementById('toast-container');
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;
    container.appendChild(toast);
    setTimeout(() => toast.remove(), 3100);
}
