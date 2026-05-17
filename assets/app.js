// @ts-check

/**
 * @import { Cook } from "../bindings/Cook"
 * @import { Equipment } from "../bindings/Equipment"
 * @import { Food } from "../bindings/Food"
 * @import { Ingredient } from "../bindings/Ingredient"
 * @import { Kitchen } from "../bindings/Kitchen"
 * @import { Material } from "../bindings/Material"
 * @import { Recipe } from "../bindings/Recipe"
 * @import { SkillLevel } from "../bindings/SkillLevel"
 * @import { Step } from "../bindings/Step"
 */

(function () {
  "use strict";

  /**
   * @typedef {Object} Defaults
   * @property {Kitchen} kitchen
   * @property {Cook[]} cooks
   * @property {Recipe[]} recipes
   */

  /**
   * @typedef {Object} State
   * @property {Defaults | null} defaults
   * @property {Record<string, boolean>} selectedEquipment
   * @property {Record<string, boolean>} selectedCooks
   * @property {Record<string, boolean>} selectedRecipes
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
   * @typedef {{ [key: string]: HTMLElement }} Views
   * @property {HTMLElement} config
   * @property {HTMLElement} loading
   * @property {HTMLElement} gantt
   * @property {HTMLElement} error
   */

  /** @type {Views} */
  var views = {
    config: /** @type {HTMLElement} */ (document.getElementById("configView")),
    loading: /** @type {HTMLElement} */ (document.getElementById("loadingView")),
    gantt: /** @type {HTMLElement} */ (document.getElementById("ganttView")),
    error: /** @type {HTMLElement} */ (document.getElementById("error-msg")),
  };

  /**
   * Marks the given view as active
   * @param {string} name
   */
  function showView(name) {
    Object.keys(views).forEach(function (/** @type {keyof Views} */ k) {
      views[k].classList.toggle("active", k === name);
    });
  }

  // Init: fetch defaults and build config UI
  fetch("/api/defaults")
    .then(function (r) {
      if (!r.ok) throw new Error("HTTP " + r.status);
      return r.json();
    })
    .then(function (data) {
      state.defaults = /** @type {Defaults} */ (data);
      buildConfigUI(data);
      showView("config");
    })
    .catch(function (err) {
      showView("error");
      var errorEl = /** @type {HTMLElement} */ (document.getElementById("error-msg"));
      errorEl.textContent = "Failed to load defaults: " + err.message;
      errorEl.style.display = "block";
    });

  /**
   * Builds the Configuration UI
   * @param {Defaults} data
   */
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
    var container = /** @type {HTMLElement} */ (document.getElementById("kitchenContent"));
    container.innerHTML = "";

    // Select all by default
    kitchen.equipment.forEach(function (/** @type {Equipment} */ eq) {
      state.selectedEquipment[eq.id] = true;
    });

    var grid = document.createElement("div");
    grid.className = "equipment-grid";

    kitchen.equipment.forEach(function (/** @type {Equipment} */ eq) {
      var el = document.createElement("div");
      el.className = "equipment-item selected";
      el.innerHTML =
        '<span class="toggle">&#10003;</span> ' +
        escapeHtml(eq.name) +
        ' <span class="kind">(' +
        escapeHtml(eq.kind) +
        ")</span>";
      el.addEventListener("click", function () {
        state.selectedEquipment[eq.id] = !state.selectedEquipment[eq.id];
        el.classList.toggle("selected");
        var toggle = el.querySelector(".toggle");
        if (toggle) toggle.textContent = state.selectedEquipment[eq.id] ? "\u2713" : "";
        updateGenerateButton();
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
    var container = /** @type {HTMLElement} */ (document.getElementById("cooksContent"));
    container.innerHTML = "";

    var list = document.createElement("div");
    list.className = "cook-list";

    cooks.forEach(function (/** @type {Cook} */ cook) {
      // Select all cooks by default
      state.selectedCooks[cook.name] = true;

      var skillStr = Object.keys(cook.skills || {})
        .map(function (/** @type {string} */ s) {
          return s + ": " + cook.skills[s];
        })
        .join(", ");

      var el = document.createElement("div");
      el.className = "cook-card selected";
      el.innerHTML =
        '<span class="checkbox">&#10003;</span>' +
        '<span class="name">' +
        escapeHtml(cook.name) +
        "</span>" +
        (skillStr
          ? '<span class="skills"> &mdash; ' + escapeHtml(skillStr) + "</span>"
          : "");
      el.addEventListener("click", function () {
        state.selectedCooks[cook.name] = !state.selectedCooks[cook.name];
        el.classList.toggle("selected");
        var checkbox = el.querySelector(".checkbox");
        if (checkbox) checkbox.textContent = state.selectedCooks[cook.name] ? "\u2713" : "";
        updateGenerateButton();
      });
      list.appendChild(el);
    });

    container.appendChild(list);
  }

  /**
   * Builds the Recipes UI
   * @param {Recipe[]} recipes
   */
  function buildRecipesUI(recipes) {
    var container = /** @type {HTMLElement} */ (document.getElementById("recipesContent"));
    container.innerHTML = "";

    // Also show uploaded recipes
    var allRecipes = recipes.concat(state.uploadedRecipes);

    var list = document.createElement("div");
    list.className = "recipe-list";

    allRecipes.forEach(function (/** @type {Recipe} */ recipe) {
      state.selectedRecipes[recipe.name] = true;

      var ingredientStr = (recipe.ingredients || [])
        .map(function (/** @type {Ingredient} */ ing) {
          return (
            ing.quantity +
            " " +
            ing.unit +
            " " +
            ing.name +
            (ing.optional ? " (optional)" : "") +
            (ing.alternatives && ing.alternatives.length
              ? " alt: " + ing.alternatives.join("/")
              : "")
          );
        })
        .join(", ");

      var el = document.createElement("div");
      el.className = "recipe-card selected";
      el.innerHTML =
        '<span class="checkbox">&#10003;</span>' +
        '<div class="recipe-info">' +
        '<div class="recipe-name">' +
        escapeHtml(recipe.name) +
        "</div>" +
        '<div class="recipe-ingredients">' +
        escapeHtml(ingredientStr) +
        "</div>" +
        "</div>";
      el.addEventListener("click", function () {
        state.selectedRecipes[recipe.name] =
          !state.selectedRecipes[recipe.name];
        el.classList.toggle("selected");
        var checkbox = el.querySelector(".checkbox");
        if (checkbox) checkbox.textContent = state.selectedRecipes[recipe.name] ? "\u2713" : "";
        updateGenerateButton();
      });
      list.appendChild(el);
    });

    // Upload zone
    var uploadZone = document.createElement("div");
    uploadZone.className = "recipe-upload";
    uploadZone.innerHTML = "<div>+ Upload JSON recipe file</div>";
    var fileInput = /** @type {HTMLInputElement} */ (document.createElement("input"));
    fileInput.type = "file";
    fileInput.accept = ".json,application/json";
    fileInput.addEventListener("change", function () {
      if (fileInput.files && fileInput.files.length > 0) {
        var reader = new FileReader();
        reader.onload = function (/** @type {ProgressEvent<FileReader>} */ e) {
          try {
            var target = e.target;
            if (!target || typeof target.result !== "string") return;
            var recipe = /** @type {Recipe} */ (JSON.parse(target.result));
            if (!recipe.name || !recipe.steps) {
              alert('Invalid recipe JSON: missing "name" or "steps"');
              return;
            }
            state.uploadedRecipes.push(recipe);
            // Rebuild recipe UI to include the uploaded recipe
            var defaults = /** @type {Defaults} */ (state.defaults);
            buildRecipesUI(defaults.recipes);
            updateGenerateButton();
          } catch (err) {
            alert("Invalid JSON: " + /** @type {Error} */ (err).message);
          }
        };
        reader.readAsText(fileInput.files[0]);
      }
    });
    uploadZone.appendChild(fileInput);
    uploadZone.addEventListener("click", function () {
      fileInput.click();
    });
    list.appendChild(uploadZone);

    container.appendChild(list);
  }

  function updateGenerateButton() {
    var btn = /** @type {HTMLButtonElement} */ (document.getElementById("btnGenerate"));
    var hasCook = Object.values(state.selectedCooks).some(function (/** @type {boolean} */ v) {
      return v;
    });
    var hasRecipe = Object.values(state.selectedRecipes).some(function (/** @type {boolean} */ v) {
      return v;
    });
    btn.disabled = !(hasCook && hasRecipe);
    updateValidationUI();
  }

  /** @type {HTMLElement} */ (document.getElementById("btnGenerate"))
    .addEventListener("click", generatePlan);

  /**
   * @returns {void}
   */
  function generatePlan() {
    var defaults = /** @type {Defaults} */ (state.defaults);

    // Build kitchen with only selected equipment
    /** @type {Kitchen} */
    var kitchen = JSON.parse(JSON.stringify(defaults.kitchen));
    kitchen.equipment = kitchen.equipment.filter(function (/** @type {Equipment} */ eq) {
      return state.selectedEquipment[eq.id];
    });

    // Build cooks list
    /** @type {Cook[]} */
    var cooks = defaults.cooks.filter(function (/** @type {Cook} */ c) {
      return state.selectedCooks[c.name];
    });

    // Build recipes list (defaults + uploaded)
    var allRecipes = defaults.recipes.concat(state.uploadedRecipes);
    /** @type {Recipe[]} */
    var recipes = allRecipes.filter(function (/** @type {Recipe} */ r) {
      return state.selectedRecipes[r.name];
    });

    var validation = validatePlanFeasibility(kitchen, cooks, recipes);
    if (!validation.feasible) {
      showView("config");
      views.error.textContent =
        "Cannot generate plan:\n" + validation.errors.join("\n");
      views.error.style.display = "block";
      return;
    }

    var payload = { kitchen: kitchen, cooks: cooks, recipes: recipes };

    showView("loading");

    fetch("/api/plan", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    })
      .then(function (/** @type {Response} */ r) {
        if (!r.ok)
          return r.json().then(function (/** @type {Record<string, unknown>} */ err) {
            var msg = /** @type {string} */ (err.error) || "HTTP " + r.status;
            if (Array.isArray(err.errors) && err.errors.length) {
              msg = err.errors.map(function (/** @type {{ message: string }} */ e) { return e.message; }).join("; ");
            }
            throw new Error(msg);
          });
        return r.json();
      })
      .then(function (/** @type {import("../bindings/Plan").Plan} */ plan) {
        renderGantt(plan, /** @type {HTMLElement} */ (document.getElementById("ganttContent")));
        var h2 = document.querySelector("#ganttView .gantt-header h2");
        if (h2) h2.textContent =
          "Plan — " +
          recipes
            .map(function (/** @type {Recipe} */ r) {
              return r.name;
            })
            .join(", ");
        showView("gantt");
      })
      .catch(function (/** @type {Error} */ err) {
        showView("config");
        views.error.textContent = "Failed to generate plan: " + err.message;
        views.error.style.display = "block";
      });
  }

  // Back to config
  /** @type {HTMLElement} */ (document.getElementById("btnBack")).addEventListener("click", function () {
    views.error.style.display = "none";
    showView("config");
  });

  /**
   * Escapes HTML special characters in a given string.
   * @param {unknown} s
   * @returns {string}
   */
  function escapeHtml(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  /**
   * Converts a skill level string to its numeric rank.
   * @param {string} skillLevel
   * @returns {number}
   */
  function numericSkill(skillLevel) {
    // NOTE: Matches SkillLevel enum in :/src/models/cook.rs
    var SKILL_LEVELS = [
      "Unskilled",
      "Novice",
      "Intermediate",
      "Advanced",
      "Expert",
    ];

    for (var i = 0; i < SKILL_LEVELS.length; i++) {
      if (SKILL_LEVELS[i] == skillLevel) return i;
    }

    return 0;
  }

  /**
   * Validates the current selection and updates the validation-errors UI.
   */
  function updateValidationUI() {
    var defaults = /** @type {Defaults} */ (state.defaults);

    // Build filtered kitchen (only selected equipment)
    /** @type {Kitchen} */
    var kitchen = JSON.parse(JSON.stringify(defaults.kitchen));
    kitchen.equipment = kitchen.equipment.filter(function (/** @type {Equipment} */ eq) {
      return state.selectedEquipment[eq.id];
    });

    // Build selected cooks
    /** @type {Cook[]} */
    var cooks = defaults.cooks.filter(function (/** @type {Cook} */ c) {
      return state.selectedCooks[c.name];
    });

    // Build selected recipes (defaults + uploaded)
    var allRecipes = defaults.recipes.concat(state.uploadedRecipes);
    /** @type {Recipe[]} */
    var recipes = allRecipes.filter(function (/** @type {Recipe} */ r) {
      return state.selectedRecipes[r.name];
    });

    var result = validatePlanFeasibility(kitchen, cooks, recipes);
    var errorsEl = document.getElementById("validation-errors");
    var btn = /** @type {HTMLButtonElement} */ (document.getElementById("btnGenerate"));

    if (result.errors.length > 0 && errorsEl) {
      errorsEl.innerHTML =
        "<ul>" +
        result.errors
          .map(function (/** @type {string} */ e) {
            return "<li>" + escapeHtml(e) + "</li>";
          })
          .join("") +
        "</ul>";
      errorsEl.style.display = "block";
      btn.disabled = true;
    } else {
      if (errorsEl) {
        errorsEl.style.display = "none";
        errorsEl.innerHTML = "";
      }
      // Re-evaluate base enable condition (button may have been disabled by validation)
      var hasCook = Object.values(state.selectedCooks).some(function (/** @type {boolean} */ v) {
        return v;
      });
      var hasRecipe = Object.values(state.selectedRecipes).some(function (/** @type {boolean} */ v) {
        return v;
      });
      btn.disabled = !(hasCook && hasRecipe);
    }
  }

  /**
   * Sanity check for plan making
   * @param {Kitchen} kitchen
   * @param {Cook[]} cooks
   * @param {Recipe[]} recipes
   * @returns {{ feasible: boolean, errors: string[] }}
   */
  /**
   * @param {Kitchen} kitchen
   * @param {Cook[]} cooks
   * @param {Recipe[]} recipes
   * @returns {{ feasible: boolean, errors: string[] }}
   */
  function validatePlanFeasibility(kitchen, cooks, recipes) {
    /** @type {string[]} */
    var errors = [];

    if (!recipes || recipes.length === 0) {
      errors.push("No recipes selected");
      return { feasible: false, errors: errors };
    }

    if (!kitchen.equipment || kitchen.equipment.length === 0) {
      errors.push("No kitchen equipment selected");
      return { feasible: false, errors: errors };
    }

    // Collect available equipment kinds
    /** @type {Record<string, boolean>} */
    var equipKinds = {};
    kitchen.equipment.forEach(function (/** @type {Equipment} */ eq) {
      equipKinds[eq.kind] = true;
    });

    recipes.forEach(function (/** @type {Recipe} */ recipe) {
      recipe.steps.forEach(function (/** @type {Step} */ step) {
        var taskId = recipe.name + ": " + step.description;
        var stepFailed = false;

        // Check that required equipment kind exists
        if (!stepFailed && step.resource_kind && !equipKinds[step.resource_kind]) {
          errors.push("No " + step.resource_kind + " available for '" + taskId + "'");
          stepFailed = true;
        }

        // Check that cooks are available for steps needing them
        if (!stepFailed && step.needs_cook && cooks.length === 0) {
          errors.push("No cooks provided but '" + taskId + "' requires one");
          stepFailed = true;
        }

        // Check that at least one cook meets the skill requirement
        if (!stepFailed && step.needs_cook && step.skill && step.min_skill_level) {
          /** @type {string} */
          var reqSkill = step.skill;
          /** @type {string} */
          var reqLevel = step.min_skill_level;
          var hasQualified = cooks.some(function (/** @type {Cook} */ c) {
            var cookLevel = c.skills[reqSkill];
            if (!cookLevel) return false;
            return numericSkill(cookLevel) >= numericSkill(reqLevel);
          });
          if (!hasQualified) {
            errors.push(
              "No cook meets the '" +
                reqSkill +
                "' >= " +
                reqLevel +
                " requirement for '" +
                taskId +
                "'",
            );
            stepFailed = true;
          }
        }
      });
    });

    return { feasible: errors.length === 0, errors: errors };
  }
})();
