'use strict';
'require view';
'require rpc';
'require poll';
'require dom';
'require ui';

var LINES = 200;

var callLog = rpc.declare({
	object: 'srunc',
	method: 'log',
	params: [ 'lines' ],
	expect: { log: '' }
});

var callServiceStatus = rpc.declare({
	object: 'srunc',
	method: 'service_status',
	expect: { '': {} }
});

// Scoped styling; translucent colours keep it readable on light/dark themes.
function logStyle() {
	return E('style', { 'type': 'text/css' }, [
		'.srunc-logbar{display:flex;align-items:center;gap:1.2em;flex-wrap:wrap;margin:.3em 0 .6em}',
		'.srunc-logbar label{display:inline-flex;align-items:center;gap:.4em;margin:0;font-weight:400}',
		'.srunc-log{margin:0;padding:.7em .9em;max-height:60vh;overflow:auto;border:1px solid rgba(128,128,128,.35);border-radius:6px;background:rgba(128,128,128,.08);font-family:ui-monospace,Menlo,Consolas,monospace;font-size:.85em;line-height:1.5;white-space:pre-wrap;word-break:break-word}',
		'.srunc-log-empty{opacity:.6;font-style:italic}',
		'.srunc-log-hint{margin-top:.5em;opacity:.6;font-size:.85em}',
		// Reuse the same pill / dot vocabulary as the Overview status so
		// the two views feel like part of the same app.
		'.srunc-pill{display:inline-flex;align-items:center;gap:.45em;padding:.28em .9em;border-radius:999px;font-weight:600;line-height:1.5;border:1px solid transparent}',
		'.srunc-on{color:#1a8f3c;background:rgba(40,160,70,.14);border-color:rgba(40,160,70,.40)}',
		'.srunc-off{color:#8a8a8a;background:rgba(128,128,128,.14);border-color:rgba(128,128,128,.38)}',
		'.srunc-dot{width:.58em;height:.58em;border-radius:50%;background:currentColor;flex:0 0 auto}'
	].join('\n'));
}

function renderServiceBadge(running) {
	return E('span', { 'class': 'srunc-pill ' + (running ? 'srunc-on' : 'srunc-off') }, [
		E('span', { 'class': 'srunc-dot' }),
		running ? _('Service running') : _('Service stopped')
	]);
}

// Copy `text` to the clipboard. Prefer the async Clipboard API; fall back
// to a hidden textarea + execCommand for LuCI installs served over plain
// HTTP (where Clipboard API requires a secure context and refuses).
function copyText(text) {
	if (window.isSecureContext && navigator.clipboard && navigator.clipboard.writeText) {
		return navigator.clipboard.writeText(text);
	}
	return new Promise(function (resolve, reject) {
		try {
			var ta = document.createElement('textarea');
			ta.value = text;
			ta.setAttribute('readonly', '');
			ta.style.position = 'fixed';
			ta.style.top = '-1000px';
			document.body.appendChild(ta);
			ta.select();
			var ok = document.execCommand('copy');
			document.body.removeChild(ta);
			ok ? resolve() : reject(new Error('copy refused'));
		} catch (e) { reject(e); }
	});
}

return view.extend({
	load: function () {
		return Promise.all([
			callLog(LINES).catch(function () { return ''; }),
			callServiceStatus().catch(function () { return { running: false }; })
		]);
	},

	render: function (loaded) {
		var initial = (loaded && loaded[0]) || '';
		var initialStatus = (loaded && loaded[1]) || { running: false };

		var paused = false;
		var lastText = initial;
		var box = E('pre', { 'class': 'srunc-log' });
		var badgeHolder = E('span', {}, renderServiceBadge(!!initialStatus.running));

		function paint(text) {
			text = (text == null) ? '' : String(text);
			lastText = text;
			// Keep the view pinned to the tail if the user is already there.
			var atBottom = (box.scrollHeight - box.scrollTop - box.clientHeight) < 24;
			if (text === '')
				dom.content(box, E('span', { 'class': 'srunc-log-empty' }, _('No log output yet.')));
			else
				dom.content(box, text);
			if (atBottom)
				box.scrollTop = box.scrollHeight;
		}

		function refreshBadge(s) {
			dom.content(badgeHolder, renderServiceBadge(!!(s && s.running)));
		}

		var refreshBtn = E('button', { 'class': 'btn cbi-button-neutral' }, [ _('Refresh') ]);
		refreshBtn.addEventListener('click', function () {
			callLog(LINES).then(paint, function () { /* ignore */ });
			callServiceStatus().then(refreshBadge, function () { /* ignore */ });
		});

		var copyBtn = E('button', { 'class': 'btn cbi-button-action', 'title': _('Copy log to clipboard') }, [ _('Copy') ]);
		copyBtn.addEventListener('click', function () {
			copyText(lastText || '').then(function () {
				ui.addNotification(null, E('p', _('Log copied to clipboard.')), 'success');
			}, function (e) {
				ui.addNotification(null,
					E('p', _('Could not copy log: %s').format(e && e.message || _('unknown error'))),
					'danger');
			});
		});

		var pauseCb = E('input', { 'type': 'checkbox' });
		pauseCb.addEventListener('change', function () { paused = pauseCb.checked; });

		// Refresh the tail every 5 s unless paused. The service-status badge
		// piggybacks the same tick so a stop/start shows up promptly.
		poll.add(function () {
			if (paused)
				return Promise.resolve();
			return Promise.all([
				callLog(LINES).then(paint, function () { /* ignore */ }),
				callServiceStatus().then(refreshBadge, function () { /* ignore */ })
			]);
		}, 5);

		paint(initial);
		box.scrollTop = box.scrollHeight;

		return E([], [
			logStyle(),
			E('h2', {}, _('srunc')),
			E('div', { 'class': 'cbi-section' }, [
				E('h3', {}, _('Log')),
				E('div', { 'class': 'srunc-logbar' }, [
					badgeHolder,
					refreshBtn,
					copyBtn,
					E('label', {}, [ pauseCb, _('Pause auto-refresh') ])
				]),
				box,
				E('div', { 'class': 'srunc-log-hint' },
					_('Recent srunc messages from the system log. Adjust how much is shown with <em>Log level</em> under Settings → Advanced.'))
			])
		]);
	},

	// No form on this view.
	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
