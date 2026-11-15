let score = 0;
let ready = false;
async function publish() {
    if (ready) await opensd.widgets.setState('scoreboard', { score });
    for (const key of opensd.keys()) key.setTitle(String(score));
}
opensd.on('init', async () => {
    const url = await opensd.widgets.open('scoreboard', 'widget');
    opensd.log(url);
    ready = true;
    await publish();
});
opensd.action('com.example.integrations.counter', {
    tap: async () => { score++; await publish(); },
    appear: publish
});
opensd.on('widgetMessage', async event => {
    if (event.widget !== 'scoreboard' || event.payload?.action !== 'increment') return;
    score++;
    await publish();
});
