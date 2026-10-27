import { createServer, type Server } from 'node:http';

/** Minimaler Ollama-Mock (synthetisch): Modellliste und Chat-Antwort „Red“. */
export async function startMockOllama(): Promise<{ port: number; server: Server }> {
	const server = createServer((req, res) => {
		res.setHeader('content-type', 'application/json');
		if (req.method === 'GET' && req.url === '/api/tags') {
			res.end(JSON.stringify({ models: [{ name: 'mock-vision:1b' }] }));
		} else if (req.method === 'POST' && req.url === '/api/chat') {
			req.resume();
			req.on('end', () => res.end(JSON.stringify({ message: { content: 'Red' } })));
		} else {
			res.statusCode = 404;
			res.end('{}');
		}
	});
	// Auf allen Schnittstellen, damit der API-Container den Host erreicht.
	await new Promise<void>((resolve) => server.listen(0, '0.0.0.0', resolve));
	const address = server.address();
	if (!address || typeof address === 'string') throw new Error('Mock ohne Port');
	return { port: address.port, server };
}
