// The fake game's window off Windows: the title comes in as ?t=, Stop asks the backend (dqc://stop) to quit.
document.getElementById('name').textContent = new URLSearchParams(location.search).get('t') || '';
document.getElementById('stop').onclick = () => { location.href = 'dqc://stop'; };
