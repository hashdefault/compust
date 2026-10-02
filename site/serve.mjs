import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';

const root = resolve(process.env.SITE_DIR || 'site/dist');
const base = '/compust/';
const port = Number(process.env.PORT || 4173);
const types = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.svg': 'image/svg+xml', '.xml': 'application/xml', '.txt': 'text/plain' };

createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (pathname === '/' || pathname === '/compust') {
      response.writeHead(302, { Location: base }).end();
      return;
    }
    const file = resolve(root, pathname.slice(base.length));
    if (!pathname.startsWith(base) || (file !== root && !file.startsWith(root + sep))) {
      response.writeHead(404).end('Not found');
      return;
    }
    const target = (await stat(file)).isDirectory() ? resolve(file, 'index.html') : file;
    const content = await readFile(target);
    response.writeHead(200, { 'Content-Type': `${types[extname(target)] || 'application/octet-stream'}; charset=utf-8` });
    response.end(content);
  } catch (error) {
    if (error.code === 'ENOENT' || error.code === 'ENOTDIR') {
      response.writeHead(404, { 'Content-Type': 'text/html; charset=utf-8' });
      response.end(await readFile(resolve(root, '404.html')).catch(() => 'Not found'));
      return;
    }
    console.error(error);
    response.writeHead(500).end('Preview server error');
  }
}).listen(port, '127.0.0.1', () => {
  console.log(`Compust preview: http://127.0.0.1:${port}${base}`);
});
