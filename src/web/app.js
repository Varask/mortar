document.addEventListener('DOMContentLoaded', () => {
    const calculateBtn = document.getElementById('calculate-btn');
    const resultsSection = document.getElementById('results');
    const errorSection = document.getElementById('error');

    calculateBtn.addEventListener('click', calculate);

    // Allow Enter key to trigger calculation
    document.querySelectorAll('input').forEach(input => {
        input.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                calculate();
            }
        });
    });
});

async function calculate() {
    const btn = document.getElementById('calculate-btn');
    const resultsSection = document.getElementById('results');
    const errorSection = document.getElementById('error');

    // Hide previous results/errors
    resultsSection.classList.add('hidden');
    errorSection.classList.add('hidden');

    // Get input values
    const mortar = {
        name: document.getElementById('mortar-name').value || 'M1',
        elevation: parseFloat(document.getElementById('mortar-elevation').value) || 0,
        x: parseFloat(document.getElementById('mortar-x').value) || 0,
        y: parseFloat(document.getElementById('mortar-y').value) || 0
    };

    const target = {
        name: document.getElementById('target-name').value || 'T1',
        elevation: parseFloat(document.getElementById('target-elevation').value) || 0,
        x: parseFloat(document.getElementById('target-x').value) || 0,
        y: parseFloat(document.getElementById('target-y').value) || 0
    };

    // Validate inputs
    if (!mortar.name.trim()) {
        showError('Le nom du mortier est requis');
        return;
    }
    if (!target.name.trim()) {
        showError('Le nom de la cible est requis');
        return;
    }

    // Show loading state
    btn.classList.add('loading');
    btn.disabled = true;

    try {
        const response = await fetch('/api/calculate', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ mortar, target })
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.error || 'Erreur de calcul');
        }

        displayResults(data);
    } catch (error) {
        showError(error.message || 'Erreur de connexion au serveur');
    } finally {
        btn.classList.remove('loading');
        btn.disabled = false;
    }
}

function displayResults(data) {
    const resultsSection = document.getElementById('results');

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
