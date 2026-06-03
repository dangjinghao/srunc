'use strict';
'require view';
'require rpc';
'require poll';
'require dom';

var LINES = 200;

var callLog = rpc.declare({
	object: 'srunc',
	method: 'log',
	params: [ 'lines' ],
	expect: { log: '' }
});

// Scoped styling; translucent colours keep it readable on light/dark themes.
function logStyle() {
	return E('style', { 'type': 'text/css' }, [
		'.srunc-logbar{display:flex;align-items:center;gap:1.2em;flex-wrap:wrap;margin:.3em 0 .6em}',
		'.srunc-logbar label{display:inline-flex;align-items:center;gap:.4em;margin:0;font-weight:400}',
		'.srunc-log{margin:0;padding:.7em .9em;max-height:60vh;overflow:auto;border:1px solid rgba(128,128,128,.35);border-radius:6px;background:rgba(128,128,128,.08);font-family:ui-monospace,Menlo,Consolas,monospace;font-size:.85em;line-height:1.5;white-space:pre-wrap;word-break:break-word}',
		'.srunc-log-empty{opacity:.6;font-style:italic}',
		'.srunc-log-hint{margin-top:.5em;opacity:.6;font-size:.85em}'
	].join('\n'));
}

return view.extend({
	load: function () {
		return callLog(LINES).catch(function () { return ''; });
	},

	render: function (initial) {
		var paused = false;
		var box = E('pre', { 'class': 'srunc-log' });

		function paint(text) {
			text = (text == null) ? '' : String(text);
			// Keep the view pinned to the tail if the user is already there.
			var atBottom = (box.scrollHeight - box.scrollTop - box.clientHeight) < 24;
			if (text === '')
				dom.content(box, E('span', { 'class': 'srunc-log-empty' }, _('No log output yet.')));
			else
				dom.content(box, text);
			if (atBottom)
				box.scrollTop = box.scrollHeight;
		}

		var refreshBtn = E('button', { 'class': 'btn cbi-button-neutral' }, [ _('Refresh') ]);
		refreshBtn.addEventListener('click', function () {
			callLog(LINES).then(paint, function () { /* ignore */ });
		});

		var pauseCb = E('input', { 'type': 'checkbox' });
		pauseCb.addEventListener('change', function () { paused = pauseCb.checked; });

		// Refresh the tail every 5 s unless paused.
		poll.add(function () {
			if (paused)
				return Promise.resolve();
			return callLog(LINES).then(paint, function () { /* ignore */ });
		}, 5);

		paint(initial);
		box.scrollTop = box.scrollHeight;

		return E([], [
			logStyle(),
			E('h2', {}, _('srunc')),
			E('div', { 'class': 'cbi-section' }, [
				E('h3', {}, _('Log')),
				E('div', { 'class': 'srunc-logbar' }, [
					refreshBtn,
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
