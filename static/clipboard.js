document.addEventListener('DOMContentLoaded', function () {
  var ICON_COPY  = document.getElementById('icon-copy').innerHTML;
  var ICON_CHECK = document.getElementById('icon-check').innerHTML;

  document.querySelectorAll('pre').forEach(function (pre) {
    var btn = document.createElement('button');
    btn.className = 'copy-btn';
    btn.innerHTML = ICON_COPY;
    btn.setAttribute('aria-label', 'Copy code to clipboard');

    btn.addEventListener('click', function () {
      var code = pre.querySelector('code');
      var text = code ? code.textContent : pre.textContent;

      function markCopied() {
        btn.innerHTML = ICON_CHECK;
        btn.classList.add('copied');
        setTimeout(function () {
          btn.innerHTML = ICON_COPY;
          btn.classList.remove('copied');
        }, 1500);
      }

      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(markCopied).catch(fallback);
      } else {
        fallback();
      }

      function fallback() {
        var ta = document.createElement('textarea');
        ta.value = text;
        ta.style.position = 'fixed';
        ta.style.opacity = '0';
        document.body.appendChild(ta);
        ta.focus();
        ta.select();
        try { document.execCommand('copy'); } catch (_) {}
        document.body.removeChild(ta);
        markCopied();
      }
    });

    pre.appendChild(btn);
  });
});