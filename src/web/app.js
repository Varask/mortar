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
            showToast(`Mortier '${name}' ajoute`, 'success');
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

async function addTarget() {
    const name = document.getElementById('target-name').value.trim();
    const elevation = parseFloat(document.getElementById('target-elevation').value) || 0;
    const x = parseFloat(document.getElementById('target-x').value) || 0;
    const y = parseFloat(document.getElementById('target-y').value) || 0;

    if (!name) {
        showToast('Le nom de la cible est requis', 'error');
        return;
    }

    try {
        const response = await fetch('/api/targets', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, elevation, x, y })
        });

        const data = await response.json();

        if (response.ok) {
            showToast(`Cible '${name}' ajoutee`, 'success');
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
        const response = await fetch('/api/calculate-by-name', {
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
                <span class="position-name">${mortar.name}</span>
                <span class="position-coords">X:${mortar.x} Y:${mortar.y} E:${mortar.elevation}m</span>
            </div>
            <button class="btn-delete" data-name="${mortar.name}">X</button>
        `;

        // Click on item to select
        li.querySelector('.position-info').addEventListener('click', () => {
            selectedMortar = mortar.name;
            document.getElementById('selected-mortar').value = mortar.name;
            renderMortarsList();
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
                <span class="position-name">${target.name}</span>
                <span class="position-coords">X:${target.x} Y:${target.y} E:${target.elevation}m</span>
            </div>
            <button class="btn-delete" data-name="${target.name}">X</button>
        `;

        // Click on item to select
        li.querySelector('.position-info').addEventListener('click', () => {
            selectedTarget = target.name;
            document.getElementById('selected-target').value = target.name;
            renderTargetsList();
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
        option.textContent = `${mortar.name} (${mortar.x}, ${mortar.y})`;
        select.appendChild(option);
    }

    // Restore selection if still valid
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
        option.textContent = `${target.name} (${target.x}, ${target.y})`;
        select.appendChild(option);
    }

    // Restore selection if still valid
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

    // Build solutions table
    const tbody = document.getElementById('solutions-body');
    tbody.innerHTML = '';

    const rings = ['0R', '1R', '2R', '3R', '4R'];
    const ammoTypes = ['PRACTICE', 'HE', 'SMOKE', 'FLARE'];

    for (const ammoType of ammoTypes) {
        const row = document.createElement('tr');

        // Ammo type cell
        const typeCell = document.createElement('td');
        typeCell.textContent = ammoType;
        row.appendChild(typeCell);

        // Ring cells
        const ammoSolutions = data.solutions[ammoType] || {};
        for (const ring of rings) {
            const cell = document.createElement('td');
            const value = ammoSolutions[ring];

            if (value !== null && value !== undefined) {
                cell.textContent = value.toFixed(1);
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
    // Remove existing toasts
    document.querySelectorAll('.toast').forEach(t => t.remove());

    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;
    document.body.appendChild(toast);

    setTimeout(() => {
        toast.remove();
    }, 3000);
}
