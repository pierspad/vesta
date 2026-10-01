#!/usr/bin/env python3
"""Unauthenticated, bounded provider/link audit. Does not run inference."""
import concurrent.futures
import json
import re
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
urls = {}
for name in ('llmProviders', 'transcribeProviders'):
    source = (ROOT / f'apps/srt-gui/src/lib/config/{name}.ts').read_text()
    for field, url in re.findall(r'(apiKeyUrl|documentationUrl|defaultApiUrl|defaultUrl): "(https://[^" ]+)"', source):
        if field in ('defaultApiUrl', 'defaultUrl'):
            # Only discovery requests; never upload audio or run inference.
            if 'assemblyai' in url or 'deepgram' in url or 'anthropic' in url or 'models.github' in url:
                continue
            url += '/models'
            field = 'model discovery (no credentials)'
        urls.setdefault(url, set()).add(field)

def probe(item):
    url, kinds = item
    request = urllib.request.Request(url, headers={'User-Agent': 'Vesta-link-audit/1.0'})
    try:
        with urllib.request.urlopen(request, timeout=12) as response:
            status, final = response.status, response.url
    except urllib.error.HTTPError as error:
        status, final = error.code, error.url
    except Exception as error:
        return {'url': url, 'kind': sorted(kinds), 'status': None, 'error': str(error), 'verdict': 'unverified'}
    verdict = 'reachable' if 200 <= status < 400 else 'authentication-or-access-required' if status in (401, 403) else 'review'
    return {'url': url, 'kind': sorted(kinds), 'status': status, 'final_url': final, 'verdict': verdict}

if __name__ == '__main__':
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
        print(json.dumps(list(executor.map(probe, sorted(urls.items()))), indent=2))
