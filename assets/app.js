// @ts-check

/**
 * @import { Cook } from "../bindings/Cook"
 * @import { Equipment } from "../bindings/Equipment"
 * @import { Food } from "../bindings/Food"
 * @import { Ingredient } from "../bindings/Ingredient"
 * @import { Kitchen } from "../bindings/Kitchen"
 * @import { Material } from "../bindings/Material"
 * @import { Plan } from "../bindings/Plan"
 * @import { Recipe } from "../bindings/Recipe"
 * @import { SkillLevel } from "../bindings/SkillLevel"
 * @import { Step } from "../bindings/Step"
 * @import { Task } from "../bindings/Task"
 */

(function () {
  'use strict';


  /**
   * @typedef {Object} State
   * @property {Object | null} defaults
   * @property {Object} selectedEquipment
   * @property {Object} selectedCooks
   * @property {Object} selectedRecipes
   * @property {Recipe[]} uploadedRecipes
   */

  /** @type {State} */
  var state = {
    defaults: null,
    selectedEquipment: {},
    selectedCooks: {},
    selectedRecipes: {},
    uploadedRecipes: [],
  };

  // View switching
  /**
   * @typedef {Object} Views
   * @property {Element | null} config
   * @property {Element | null} loading
   * @property {Element | null} gantt
   * @property {Element | null} error
   */

  /** @type {Views} */
  var views = {
    config: document.getElementById('configView'),
    loading: document.getElementById('loadingView'),
    gantt: document.getElementById('ganttView'),
    error: document.getElementById('error-msg'),
  };

  function showView(name) {
    Object.keys(views).forEach(function (k) {
      views[k].classList.toggle('active', k === name);
    });
  }

  // Init: fetch defaults and build config UI
  fetch('/api/defaults')
    .then(function (r) {
      if (!r.ok) throw new Error('HTTP ' + r.status);
      return r.json();
    })
    .then(function (data) {
      state.defaults = data;
      buildConfigUI(data);
      showView('config');
    })
    .catch(function (err) {
      showView('error');
      document.getElementById('error-msg').textContent = 'Failed to load defaults: ' + err.message;
      document.getElementById('error-msg').style.display = 'block';
    });

  function buildConfigUI(data) {
    buildKitchenUI(data.kitchen);
    buildCooksUI(data.cooks);
    buildRecipesUI(data.recipes);
    updateGenerateButton();
  }

  /**
   * Builds the kitchen UI
   * @param {Kitchen} kitchen
   */
  function buildKitchenUI(kitchen) {
    var container = document.getElementById('kitchenContent');
    container.innerHTML = '';

    // Select all by default
    kitchen.equipment.forEach(function (eq) {
      state.selectedEquipment[eq.id] = true;
    });

    var grid = document.createElement('div');
    grid.className = 'equipment-grid';

    kitchen.equipment.forEach(function (eq) {
      var el = document.createElement('div');
      el.className = 'equipment-item selected';
      el.innerHTML = '<span class="toggle">&#10003;</span> '
        + escapeHtml(eq.name)
        + ' <span class="kind">(' + escapeHtml(eq.kind) + ')</span>';
      el.addEventListener('click', function () {
        state.selectedEquipment[eq.id] = !state.selectedEquipment[eq.id];
        el.classList.toggle('selected');
        el.querySelector('.toggle').textContent = state.selectedEquipment[eq.id] ? '\u2713' : '';
      });
      grid.appendChild(el);
    });

    container.appendChild(grid);
  }

  /**
   * Builds the Cooks UI
   * @param {Cook[]} cooks
   */
  function buildCooksUI(cooks) {
    var container = document.getElementById('cooksContent');
    container.innerHTML = '';

    var list = document.createElement('div');
    list.className = 'cook-list';

    cooks.forEach(function (cook) {
      // Select all cooks by default
      state.selectedCooks[cook.name] = true;

      var skillStr = Object.keys(cook.skills || {}).map(function (s) {
        return s + ': ' + cook.skills[s];
      }).join(', ');

      var el = document.createElement('div');
      el.className = 'cook-card selected';
      el.innerHTML = '<span class="checkbox">&#10003;</span>'
        + '<span class="name">' + escapeHtml(cook.name) + '</span>'
        + (skillStr ? '<span class="skills"> &mdash; ' + escapeHtml(skillStr) + '</span>' : '');
      el.addEventListener('click', function () {
        state.selectedCooks[cook.name] = !state.selectedCooks[cook.name];
        el.classList.toggle('selected');
        el.querySelector('.checkbox').textContent = state.selectedCooks[cook.name] ? '\u2713' : '';
        updateGenerateButton();
      });
      list.appendChild(el);
    });

    container.appendChild(list);
  }

  /**
   * Builds the Recipes UI
   * @param {Recipes[]} recipes
   */
  function buildRecipesUI(recipes) {
    var container = document.getElementById('recipesContent');
    container.innerHTML = '';

    // Also show uploaded recipes
    var allRecipes = recipes.concat(state.uploadedRecipes);

    var list = document.createElement('div');
    list.className = 'recipe-list';

    allRecipes.forEach(function (recipe) {
      state.selectedRecipes[recipe.name] = true;

      var ingredientStr = (recipe.ingredients || []).map(function (ing) {
        return ing.quantity + ' ' + ing.unit + ' ' + ing.name
          + (ing.optional ? ' (optional)' : '')
          + (ing.alternatives && ing.alternatives.length ? ' alt: ' + ing.alternatives.join('/') : '');
      }).join(', ');

      var el = document.createElement('div');
      el.className = 'recipe-card selected';
      el.innerHTML = '<span class="checkbox">&#10003;</span>'
        + '<div class="recipe-info">'
        + '<div class="recipe-name">' + escapeHtml(recipe.name) + '</div>'
        + '<div class="recipe-ingredients">' + escapeHtml(ingredientStr) + '</div>'
        + '</div>';
      el.addEventListener('click', function () {
        state.selectedRecipes[recipe.name] = !state.selectedRecipes[recipe.name];
        el.classList.toggle('selected');
        el.querySelector('.checkbox').textContent = state.selectedRecipes[recipe.name] ? '\u2713' : '';
        updateGenerateButton();
      });
      list.appendChild(el);
    });

    // Upload zone
    var uploadZone = document.createElement('div');
    uploadZone.className = 'recipe-upload';
    uploadZone.innerHTML = '<div>+ Upload JSON recipe file</div>';
    var fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.accept = '.json,application/json';
    fileInput.addEventListener('change', function () {
      if (fileInput.files.length > 0) {
        var reader = new FileReader();
        reader.onload = function (e) {
          try {
            var recipe = JSON.parse(e.target.result);
            if (!recipe.name || !recipe.steps) {
              alert('Invalid recipe JSON: missing "name" or "steps"');
              return;
            }
            state.uploadedRecipes.push(recipe);
            // Rebuild recipe UI to include the uploaded recipe
            buildRecipesUI(state.defaults.recipes);
          } catch (err) {
            alert('Invalid JSON: ' + err.message);
          }
        };
        reader.readAsText(fileInput.files[0]);
      }
    });
    uploadZone.appendChild(fileInput);
    uploadZone.addEventListener('click', function () { fileInput.click(); });
    list.appendChild(uploadZone);

    container.appendChild(list);
  }

  function updateGenerateButton() {
    var btn = document.getElementById('btnGenerate');
    var hasCook = Object.values(state.selectedCooks).some(function (v) { return v; });
    var hasRecipe = Object.values(state.selectedRecipes).some(function (v) { return v; });
    btn.disabled = !(hasCook && hasRecipe);
  }

  document.getElementById('btnGenerate').addEventListener('click', generatePlan);

  /**
   * Returns the sum of a and b
   * @returns {Promise<Plan>} Promise of a Plan
   */
  function generatePlan() {
    // Build kitchen with only selected equipment
    /**
     * @type {Kitchen}
     */
    var kitchen = JSON.parse(JSON.stringify(state.defaults.kitchen));
    kitchen.equipment = kitchen.equipment.filter(function (eq) {
      return state.selectedEquipment[eq.id];
    });

    // Build cooks list
    var allCooks = state.defaults.cooks;
    /**
     * @type {Cook[]}
     */
    var cooks = allCooks.filter(function (c) {
      return state.selectedCooks[c.name];
    });

    // Build recipes list (defaults + uploaded)
    var allRecipes = state.defaults.recipes.concat(state.uploadedRecipes);
    /**
     * @type {Recipe[]}
     */
    var recipes = allRecipes.filter(function (r) {
      return state.selectedRecipes[r.name];
    });

    var payload = { kitchen: kitchen, cooks: cooks, recipes: recipes };

    showView('loading');

    fetch('/api/plan', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })
      .then(function (r) {
        if (!r.ok) return r.json().then(function (err) { throw new Error(err.error || 'HTTP ' + r.status); });
        return r.json();
      })
      .then(function (plan) {
        renderGantt(plan, document.getElementById('ganttContent'));
        document.querySelector('#ganttView .gantt-header h2').textContent =
          'Plan — ' + recipes.map(function (r) { return r.name; }).join(', ');
        showView('gantt');
      })
      .catch(function (err) {
        showView('config');
        views.error.textContent = 'Failed to generate plan: ' + err.message;
        views.error.style.display = 'block';
      });
  }

  // Back to config
  document.getElementById('btnBack').addEventListener('click', function () {
    views.error.style.display = 'none';
    showView('config');
  });

  /**
   * Escapes HTML special characters in a given string.
   * @param {String} s
   * @returns {String} Escaped string
   */
  function escapeHtml(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }
  }
})();
