import base64
import aiohttp
import asyncio

queue = asyncio.Queue()


def get_base_data(file: str) -> str:
    with open(file, "rb") as file:
        data = file.read()
        return base64.standard_b64encode(data).decode("utf-8")


async def task():
    data = get_base_data("./samples/news.png")
    async with aiohttp.ClientSession() as sess:
        while not queue.empty():
            _ = await queue.get()
            async with sess.post(
                "http://127.0.0.1:7070/admin/create/image",
                json={"format": "png", "data": data}
            ) as resp:
                assert resp.status == 200
                await asyncio.sleep(0.2)


async def main():
    for _ in range(200_000):
        queue.put_nowait(None)

    tasks = [task() for _ in range(1)]
    t = asyncio.ensure_future(asyncio.gather(*tasks))

    while not queue.empty() and not t.done():
        print(f"currently, {queue.qsize()} in queue")
        await asyncio.sleep(1)


if __name__ == '__main__':
    asyncio.run(main())
