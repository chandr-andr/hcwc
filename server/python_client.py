#!/usr/bin/env python3
"""websocket cmd client for web_ws.py example."""

import argparse
import asyncio
import sys
from contextlib import suppress

import aiohttp  # type: ignore[import-not-found]


async def start_client(url: str) -> None:
    async def dispatch(ws: aiohttp.ClientWebSocketResponse) -> None:
        while True:
            msg = await ws.receive()

            if msg.type == aiohttp.WSMsgType.TEXT:
                print("Received Text: ", msg.data.strip())
            elif msg.type == aiohttp.WSMsgType.BINARY:
                print("Received Binary: ", msg.data)
            elif msg.type == aiohttp.WSMsgType.PING:
                await ws.pong()
            elif msg.type == aiohttp.WSMsgType.PONG:
                pass
            else:
                if msg.type == aiohttp.WSMsgType.CLOSE:
                    await ws.close()
                elif msg.type == aiohttp.WSMsgType.ERROR:
                    print("Received Error during receive %s" % ws.exception())
                elif msg.type == aiohttp.WSMsgType.CLOSED:
                    pass

                break

    async with aiohttp.ClientSession() as session:
        async with session.ws_connect(url, autoclose=False, autoping=False) as ws:
            # send request
            dispatch_task = asyncio.create_task(dispatch(ws))

            # Exit with Ctrl+D
            while line := await asyncio.to_thread(sys.stdin.readline):  # type: ignore[attr-defined]
                if line.startswith("/"):
                    await ws.send_str(line)
                else:
                    await ws.ping()
                    await ws.send_str(line)

            dispatch_task.cancel()
            with suppress(asyncio.CancelledError):
                await dispatch_task


ARGS = argparse.ArgumentParser(
    description="websocket console client for wssrv.py example."
)
ARGS.add_argument(
    "--host", action="store", dest="host", default="127.0.0.1", help="Host name"
)
ARGS.add_argument(
    "--port", action="store", dest="port", default=8080, type=int, help="Port number"
)

if __name__ == "__main__":
    args = ARGS.parse_args()
    if ":" in args.host:
        args.host, port = args.host.split(":", 1)
        args.port = int(port)

    url = f"http://{args.host}:{args.port}/ws/"
    asyncio.run(start_client(url))