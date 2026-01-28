// State
let mortars = [];
let targets = [];
let selectedMortar = null;
let selectedTarget = null;

document.addEventListener('DOMContentLoaded', () => {
    // Load initial data
    loadMortars();
    loadTargets();

    // Event listeners
    document.getElementById('add-mortar-btn').addEventListener('click', addMortar);
    document.getElementById('add-target-btn').addEventListener('click', addTarget);
    document.getElementById('calculate-btn').addEventListener('click', calculate);
    document.getElementById('apply-correction-btn').addEventListener('click', applyCorrection);

    // Selection dropdowns
    document.getElementById('selected-mortar').addEventListener('change', (e) => {
        selectedMortar = e.target.value || null;
        updateListSelection('mortars');
    });

    document.getElementById('selected-target').addEventListener('change', (e) => {
        selectedTarget = e.target.value || null;
        updateListSelection('targets');
    });

    // Enter key support for forms
    document.querySelectorAll('#mortar-name, #mortar-elevation, #mortar-x, #mortar-y').forEach(input => {
        input.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') addMortar();
        });
    });

    document.querySelectorAll('#target-name, #target-elevation, #target-x, #target-y').forEach(input => {
        input.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') addTarget();
        });
    });
});

// =====================
// API calls
// =====================
async function loadMortars() {
    try {
        const response = await fetch('/api/mortars');
        const data = await response.json();
        mortars = data.positions || [];
        renderMortarsList();
        updateMortarsDropdown();
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
    } catch (error) {
        console.error('Failed to load targets:', error);
    }
}

async function addMortar() {
    const name = document.getElementById('mortar-name').value.trim();
    const elevation = parseFloat(document.getElementById('mortar-elevation').value) || 0;
    const x = parseFloat(document.getElementById('mortar-x').value) || 0;
    const y = parseFloat(document.getElementById('mortar-y').value) || 0;
    const ammo_type = document.getElementById('mortar-ammo').value;

    if (!name) {
        showToast('Le nom du mortier est requis', 'error');
        return;
    }

    try {
        const response = await fetch('/api/mortars', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, elevation, x, y, ammo_type })
        });

        const data = await response.json();

        if (response.ok) {
            showToast(`Mortier '${name}' ajoute [${ammo_type}]`, 'success');
            document.getElementById('mortar-name').value = '';
            document.getElementById('mortar-elevation').value = '0';
            document.getElementById('mortar-x').value = '0';
            document.getElementById('mortar-y').value = '0';
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
            showToast(`Mortier '${name}' supprime`, 'success');
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

async function updateMortarAmmo(name, ammo_type) {
    try {
        const response = await fetch('/api/mortars/ammo', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, ammo_type })
        });

        if (response.ok) {
            showToast(`Ogive ${name} -> ${ammo_type}`, 'success');
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

    if (!name) {
        showToast('Le nom de la cible est requis', 'error');
        return;
    }

    try {
        const response = await fetch('/api/targets', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, elevation, x, y, target_type })
        });

        const data = await response.json();

        if (response.ok) {
            showToast(`Cible '${name}' ajoutee [${target_type}]`, 'success');
            document.getElementById('target-name').value = '';
            document.getElementById('target-elevation').value = '0';
            document.getElementById('target-x').value = '0';
            document.getElementById('target-y').value = '0';
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
            showToast(`Cible '${name}' supprimee`, 'success');
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
            showToast(`Type ${name} -> ${target_type}`, 'success');
            loadTargets();
        }
    } catch (error) {
        showToast('Erreur de connexion', 'error');
    }
}

async function applyCorrection() {
    if (!selectedTarget) {
        showToast('Selectionnez une cible d\'abord', 'error');
        return;
    }

    const vertical_m = parseFloat(document.getElementById('correction-vertical').value) || 0;
    const horizontal_m = parseFloat(document.getElementById('correction-horizontal').value) || 0;

    if (vertical_m === 0 && horizontal_m === 0) {
        showToast('Entrez une deviation a corriger', 'error');
        return;
    }

    try {
        const response = await fetch('/api/targets/correct', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                target_name: selectedTarget,
                vertical_m,
                horizontal_m
            })
        });

        const data = await response.json();

        if (response.ok) {
            showToast(`Correction appliquee: ${data.corrected}`, 'success');

            // Reset correction inputs
            document.getElementById('correction-vertical').value = '0';
            document.getElementById('correction-horizontal').value = '0';

            // Reload targets and select the corrected one
            await loadTargets();

            // Select the corrected target
            selectedTarget = data.corrected;
            document.getElementById('selected-target').value = data.corrected;
            renderTargetsList();

            // Recalculate with corrected target
            if (selectedMortar) {
                calculate();
            }
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

    // Hide previous results/errors
    resultsSection.classList.add('hidden');
    errorSection.classList.add('hidden');

    if (!selectedMortar || !selectedTarget) {
        showError('Selectionnez un mortier et une cible');
        return;
    }

    // Show loading state
    btn.classList.add('loading');
    btn.disabled = true;

    try {
        const response = await fetch('/api/calculate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                mortar_name: selectedMortar,
                target_name: selectedTarget
            })
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.error || 'Erreur de calcul');
        }

        displayResults(data, selectedMortar, selectedTarget);
    } catch (error) {
        showError(error.message || 'Erreur de connexion au serveur');
    } finally {
        btn.classList.remove('loading');
        btn.disabled = false;
    }
}

// =====================
// UI Rendering
// =====================
function renderMortarsList() {
    const list = document.getElementById('mortars-list');
    list.innerHTML = '';

    if (mortars.length === 0) {
        list.innerHTML = '<li class="empty-message">Aucun mortier</li>';
        return;
    }

    for (const mortar of mortars) {
        const li = document.createElement('li');
        li.className = selectedMortar === mortar.name ? 'selected' : '';
        li.innerHTML = `
            <div class="position-info" data-name="${mortar.name}">
                <span class="position-name">${mortar.name} <small class="ammo-badge">[${mortar.ammo_type}]</small></span>
                <span class="position-coords">X:${mortar.x} Y:${mortar.y} E:${mortar.elevation}m</span>
            </div>
            <div class="item-actions">
                <select class="ammo-select" data-name="${mortar.name}">
                    <option value="HE" ${mortar.ammo_type === 'He' ? 'selected' : ''}>HE</option>
                    <option value="PRACTICE" ${mortar.ammo_type === 'Practice' ? 'selected' : ''}>PRACTICE</option>
                    <option value="SMOKE" ${mortar.ammo_type === 'Smoke' ? 'selected' : ''}>SMOKE</option>
                    <option value="FLARE" ${mortar.ammo_type === 'Flare' ? 'selected' : ''}>FLARE</option>
                </select>
                <button class="btn-delete" data-name="${mortar.name}">X</button>
            </div>
        `;

        // Click on item to select
        li.querySelector('.position-info').addEventListener('click', () => {
            selectedMortar = mortar.name;
            document.getElementById('selected-mortar').value = mortar.name;
            renderMortarsList();
        });

        // Ammo type change
        li.querySelector('.ammo-select').addEventListener('change', (e) => {
            e.stopPropagation();
            updateMortarAmmo(mortar.name, e.target.value);
        });

        // Delete button
        li.querySelector('.btn-delete').addEventListener('click', (e) => {
            e.stopPropagation();
            deleteMortar(mortar.name);
        });

        list.appendChild(li);
    }
}

function renderTargetsList() {
    const list = document.getElementById('targets-list');
    list.innerHTML = '';

    if (targets.length === 0) {
        list.innerHTML = '<li class="empty-message">Aucune cible</li>';
        return;
    }

    for (const target of targets) {
        const li = document.createElement('li');
        li.className = selectedTarget === target.name ? 'selected' : '';
        li.innerHTML = `
            <div class="position-info" data-name="${target.name}">
                <span class="position-name">${target.name} <small class="type-badge">[${target.target_type}]</small></span>
                <span class="position-coords">X:${target.x} Y:${target.y} E:${target.elevation}m</span>
            </div>
            <div class="item-actions">
                <select class="type-select" data-name="${target.name}">
                    <option value="INFANTERIE" ${target.target_type === 'Infanterie' ? 'selected' : ''}>INF</option>
                    <option value="VEHICULE" ${target.target_type === 'Vehicule' ? 'selected' : ''}>VEH</option>
                    <option value="SOUTIEN" ${target.target_type === 'Soutien' ? 'selected' : ''}>SOU</option>
                </select>
                <button class="btn-delete" data-name="${target.name}">X</button>
            </div>
        `;

        // Click on item to select
        li.querySelector('.position-info').addEventListener('click', () => {
            selectedTarget = target.name;
            document.getElementById('selected-target').value = target.name;
            renderTargetsList();
        });

        // Target type change
        li.querySelector('.type-select').addEventListener('change', (e) => {
            e.stopPropagation();
            updateTargetType(target.name, e.target.value);
        });

        // Delete button
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

    select.innerHTML = '<option value="">-- Selectionner --</option>';
    for (const mortar of mortars) {
        const option = document.createElement('option');
        option.value = mortar.name;
        option.textContent = `${mortar.name} [${mortar.ammo_type}]`;
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

    select.innerHTML = '<option value="">-- Selectionner --</option>';
    for (const target of targets) {
        const option = document.createElement('option');
        option.value = target.name;
        option.textContent = `${target.name} [${target.target_type}]`;
        select.appendChild(option);
    }

    if (currentValue && targets.some(t => t.name === currentValue)) {
        select.value = currentValue;
    } else {
        selectedTarget = null;
    }
}

function updateListSelection(type) {
    if (type === 'mortars') {
        renderMortarsList();
    } else {
        renderTargetsList();
    }
}

function displayResults(data, mortarName, targetName) {
    const resultsSection = document.getElementById('results');

    // Update header
    document.getElementById('solution-mortar').textContent = mortarName;
    document.getElementById('solution-target').textContent = targetName;

    // Update metrics
    document.getElementById('distance').textContent = data.distance_m.toFixed(1);
    document.getElementById('azimuth').textContent = data.azimuth_deg.toFixed(1);
    document.getElementById('elevation-diff').textContent = data.elevation_diff_m.toFixed(1);

    // Update type info
    document.getElementById('mortar-ammo-display').textContent = data.mortar_ammo;
    document.getElementById('target-type-display').textContent = data.target_type;
    document.getElementById('recommended-ammo').textContent = data.recommended_ammo;

    // Update selected solution cards
    if (data.selected_solution) {
        document.getElementById('selected-ammo-type').textContent = data.selected_solution.ammo_type;
        const cardsContainer = document.getElementById('elevation-cards');
        cardsContainer.innerHTML = '';

        const rings = ['0R', '1R', '2R', '3R', '4R'];
        for (const ring of rings) {
            const elev = data.selected_solution.elevations[ring];
            const disp = data.selected_solution.dispersions ? data.selected_solution.dispersions[ring] : null;
            const card = document.createElement('div');
            card.className = 'elevation-card';
            card.innerHTML = `
                <div class="ring">${ring}</div>
                <div class="value ${elev === null ? 'na' : ''}">${elev !== null ? elev.toFixed(1) : 'N/A'}</div>
                <div class="dispersion ${disp === null ? 'na' : ''}">±${disp !== null ? disp.toFixed(1) : '--'}m</div>
            `;
            cardsContainer.appendChild(card);
        }
    }

    // Build all solutions table
    const tbody = document.getElementById('solutions-body');
    tbody.innerHTML = '';

    const rings = ['0R', '1R', '2R', '3R', '4R'];
    const ammoTypes = ['PRACTICE', 'HE', 'SMOKE', 'FLARE'];

    for (const ammoType of ammoTypes) {
        const row = document.createElement('tr');
        if (ammoType === data.mortar_ammo) {
            row.classList.add('highlighted-row');
        }

        const typeCell = document.createElement('td');
        typeCell.textContent = ammoType;
        row.appendChild(typeCell);

        const ammoSolutions = data.solutions[ammoType] || {};
        const ammoDispersions = data.dispersions ? (data.dispersions[ammoType] || {}) : {};
        for (const ring of rings) {
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
}

function showError(message) {
    const errorSection = document.getElementById('error');
    errorSection.textContent = message;
    errorSection.classList.remove('hidden');
}

function showToast(message, type = 'success') {
    document.querySelectorAll('.toast').forEach(t => t.remove());

    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;
    document.body.appendChild(toast);

    setTimeout(() => {
        toast.remove();
    }, 3000);
}
