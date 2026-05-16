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

function escapeHtml(s) {
  return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

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
    th.addEventListener('click', function () {
      renderBody(plan, this.dataset.sort);
    });
  });

  // Hover highlighting
  table.addEventListener('mouseover', function (e) {
    var row = e.target.closest('tr[data-task-id]');
    if (!row) return;
    var taskId = row.dataset.taskId;
    var deps = (row.dataset.dependsOn || '').split(/,\s*/).filter(Boolean);
    deps.forEach(function (id) {
      var dep = document.querySelector('tr[data-task-id="' + id + '"]');
      if (dep) dep.classList.add('dep-upstream');
    });
    document.querySelectorAll('tr[data-depends-on]').forEach(function (other) {
      var otherDeps = (other.dataset.dependsOn || '').split(/,\s*/).filter(Boolean);
      if (otherDeps.indexOf(taskId) !== -1) other.classList.add('dep-downstream');
    });
  });
  table.addEventListener('mouseout', function (e) {
    var row = e.target.closest('tr[data-task-id]');
    if (!row) return;
    document.querySelectorAll('.dep-upstream, .dep-downstream').forEach(function (el) {
      el.classList.remove('dep-upstream', 'dep-downstream');
    });
  });
}

function renderBody(plan, sortOrder) {
  var tasks = plan.tasks.slice();
  if (sortOrder === 'cook') {
    tasks.sort(function (a, b) {
      var ca = a.cook || '';
      var cb = b.cook || '';
      if (ca < cb) return -1;
      if (ca > cb) return 1;
      return a.start_offset_minutes - b.start_offset_minutes;
    });
  } else {
    tasks.sort(function (a, b) {
      return a.start_offset_minutes - b.start_offset_minutes;
    });
  }

  var totalDuration = tasks.reduce(function (max, t) {
    return Math.max(max, t.start_offset_minutes + t.duration_minutes);
  }, 0) || 1;
  var totalF = totalDuration;

  var dishEnd = {};
  for (var i = 0; i < tasks.length; i++) {
    var t = tasks[i];
    var end = t.start_offset_minutes + t.duration_minutes;
    if (!(t.dish in dishEnd) || end > dishEnd[t.dish]) dishEnd[t.dish] = end;
  }

  var rows = '';
  for (var i = 0; i < tasks.length; i++) {
    var t = tasks[i];
    var start = t.start_offset_minutes;
    var end = start + t.duration_minutes;
    var offsetPct = (start / totalF * 100).toFixed(0);
    var widthPct = Math.max(1, (t.duration_minutes / totalF * 100).toFixed(0));
    var barLabel = start + '\u2013' + end;
    var taskId = escapeHtml(t.id || '');
    var depIds = (t.dependencies || []).map(escapeHtml).join(', ');
    var dish = escapeHtml(t.dish || '');
    var desc = escapeHtml(t.description);
    var cook = t.cook || '(none)';
    var escapedCook = escapeHtml(cook);
    var resourceKind = t.resource_kind || null;
    var resourceName = t.resource_id || null;
    var resource = resourceKind && resourceName
      ? resourceKind + ' (' + resourceName + ')'
      : escapeHtml(resourceKind || resourceName || '(none)');

    var cookColorVal = cookColor(cook);
    var cookCell = cookColorVal
      ? '<span class="cook-badge" style="background:' + cookColorVal + ';">' + escapedCook + '</span>'
      : escapedCook;

    var barClass = 'bar' + (end >= dishEnd[t.dish] ? ' bar-last' : '');
    rows += '<tr data-task-id="' + taskId + '" data-depends-on="' + depIds + '">'
      + '<td><div class="bar-container"><div class="' + barClass + '" style="margin-left:' + offsetPct + '%;width:' + widthPct + '%;" title="' + barLabel + '"></div></div></td>'
      + '<td>' + dish + '</td>'
      + '<td>' + desc + '</td>'
      + '<td>' + cookCell + '</td>'
      + '<td>' + resource + '</td>'
      + '</tr>\n';
  }

  document.getElementById('ganttBody').innerHTML = rows;
}
