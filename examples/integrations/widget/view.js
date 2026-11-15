opensdWidget.subscribe(state => {
    document.getElementById('score').textContent = String(state?.score ?? 0);
});
document.getElementById('score').onclick = () => opensdWidget.send({ action: 'increment' });
