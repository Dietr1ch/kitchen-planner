// @ts-check

// Types Plan and Task are imported inline as needed.

/**
 * @param {string} name
 * @returns {string | null}
 */
function cookColor(name) {
  if (name === '(none)') return null;
  var hash = 0;
  for (var i = 0; i < name.length; i++) {
    hash = ((hash << 5) - hash) + name.charCodeAt(i);
    hash |= 0;
  }
  var hue = ((hash % 360) + 360) % 360;
  return 'hsl(' + hue + ', 60%, 50%)';
}

/**
 * @param {unknown} s
 * @returns {string}
 */
function escapeHtml(s) {
  return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

/**
 * @param {import("../bindings/Plan").Plan} plan
 * @param {HTMLElement} container
 */
function renderGantt(plan, container) {
  container.innerHTML = '';

  var table = document.createElement('table');
  table.id = 'ganttTable';

  var thead = document.createElement('thead');
  thead.innerHTML = '<tr>'
    + '<th data-sort="start">Duration ↕</th>'
    + '<th>Dish</th>'
    + '<th>Task</th>'
    + '<th data-sort="cook">Cook ↕</th>'
    + '<th>Resource</th>'
    + '</tr>';
  table.appendChild(thead);

  var tbody = document.createElement('tbody');
  tbody.id = 'ganttBody';
  table.appendChild(tbody);
  container.appendChild(table);

  renderBody(plan, 'start');

  // Column header click sorting
  table.querySelectorAll('th[data-sort]').forEach(function (th) {
    /** @type {HTMLElement} */ (th).addEventListener('click', function () {
      renderBody(plan, /** @type {HTMLElement} */ (this).dataset.sort || 'start');
    });
  });

  // Hover highlighting
  table.addEventListener('mouseover', function (e) {
    var target = /** @type {HTMLElement} */ (e.target);
    var row = /** @type {HTMLElement | null} */ (target.closest('tr[data-task-id]'));
    if (!row) return;
    var taskId = row.dataset.taskId || '';
    var deps = (row.dataset.dependsOn || '').split(/,\s*/).filter(Boolean);
    deps.forEach(function (/** @type {string} */ id) {
      var dep = document.querySelector('tr[data-task-id="' + id + '"]');
      if (dep) dep.classList.add('dep-upstream');
    });
    document.querySelectorAll('tr[data-depends-on]').forEach(function (other) {
      var otherEl = /** @type {HTMLElement} */ (other);
      var otherDeps = (otherEl.dataset.dependsOn || '').split(/,\s*/).filter(Boolean);
      if (otherDeps.indexOf(taskId) !== -1) otherEl.classList.add('dep-downstream');
    });
  });
  table.addEventListener('mouseout', function (e) {
    var target = /** @type {HTMLElement} */ (e.target);
    var row = /** @type {HTMLElement | null} */ (target.closest('tr[data-task-id]'));
    if (!row) return;
    document.querySelectorAll('.dep-upstream, .dep-downstream').forEach(function (el) {
      /** @type {HTMLElement} */ (el).classList.remove('dep-upstream', 'dep-downstream');
    });
  });
}

/**
 * @param {import("../bindings/Plan").Plan} plan
 * @param {string} sortOrder
 */
function renderBody(plan, sortOrder) {
  var tasks = plan.tasks.slice();
  if (sortOrder === 'cook') {
    tasks.sort(function (/** @type {import("../bindings/Task").Task} */ a, /** @type {import("../bindings/Task").Task} */ b) {
      var ca = a.cook || '';
      var cb = b.cook || '';
      if (ca < cb) return -1;
      if (ca > cb) return 1;
      return a.start_offset_minutes - b.start_offset_minutes;
    });
  } else {
    tasks.sort(function (/** @type {import("../bindings/Task").Task} */ a, /** @type {import("../bindings/Task").Task} */ b) {
      return a.start_offset_minutes - b.start_offset_minutes;
    });
  }

  var totalDuration = tasks.reduce(function (/** @type {number} */ max, /** @type {import("../bindings/Task").Task} */ t) {
    return Math.max(max, t.start_offset_minutes + t.duration_minutes);
  }, 0) || 1;
  var totalF = totalDuration;

  /** @type {Record<string, number>} */
  var dishEnd = {};
  for (var i = 0; i < tasks.length; i++) {
    var t = tasks[i];
    var end = t.start_offset_minutes + t.duration_minutes;
    if (!(t.dish in dishEnd) || end > dishEnd[t.dish || '']) dishEnd[t.dish || ''] = end;
  }

  var rows = '';
  for (var i = 0; i < tasks.length; i++) {
    var t = tasks[i];
    var start = t.start_offset_minutes;
    var end = start + t.duration_minutes;
    var offsetPct = (start / totalF * 100).toFixed(0);
    var widthPct = Math.max(1, Number((t.duration_minutes / totalF * 100).toFixed(0)));
    var barLabel = start + '\u2013' + end;
    var taskId = escapeHtml(t.id || '');
    var depIds = (t.dependencies || []).map(escapeHtml).join(', ');
    var dish = escapeHtml(t.dish || '');
    var desc = escapeHtml(t.description);
    var cook = t.cook || '(none)';
    var escapedCook = escapeHtml(cook);
    var resourceKinds = t.resource_kinds || [];
    var resourceIds = t.resource_ids || [];
    var resource = resourceKinds.map(function(k, i) {
      var name = resourceIds[i] || null;
      return name ? k + ' (' + name + ')' : k;
    }).join(', ') || '(none)';

    var cookColorVal = cookColor(cook);
    var cookCell = cookColorVal
      ? '<span class="cook-badge" style="background:' + cookColorVal + ';">' + escapedCook + '</span>'
      : escapedCook;

    var barClass = 'bar' + (end >= dishEnd[t.dish || ''] ? ' bar-last' : '');
    rows += '<tr data-task-id="' + taskId + '" data-depends-on="' + depIds + '">'
      + '<td><div class="bar-container"><div class="' + barClass + '" style="margin-left:' + offsetPct + '%;width:' + widthPct + '%;" title="' + barLabel + '"></div></div></td>'
      + '<td>' + dish + '</td>'
      + '<td>' + desc + '</td>'
      + '<td>' + cookCell + '</td>'
      + '<td>' + resource + '</td>'
      + '</tr>\n';
  }

  var bodyEl = document.getElementById('ganttBody');
  if (bodyEl) bodyEl.innerHTML = rows;
}
