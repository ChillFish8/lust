import base64
import requests
import uuid


def get_base_data(file: str) -> str:
    with open(file, "rb") as file:
        data = file.read()
        print(f"original {len(data)}")
        return base64.standard_b64encode(data).decode("utf-8")


def test_png_upload1():
    data = get_base_data("./samples/news.png")
    payload = {
        "format": "png",
        "data": data,
    }
    r = requests.post("http://127.0.0.1:7070/admin/create/file", json=payload)
    print(r.json())
    assert r.status_code == 200


def test_get_img1():
    r = requests.get(f"http://127.0.0.1:7070/images/c4f33387-792d-4a59-a91a-eb86a000dbc2")
    assert r.status_code == 200


def test_get_img2():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}")
    assert r.status_code == 404


def test_remove_img1():
    r = requests.delete(f"http://127.0.0.1:7070/admin/delete/a3856be9-441d-4f07-9151-85ab1c89e15d")
    print(r.content)
    assert r.status_code == 200


if __name__ == '__main__':
    test_png_upload1()
    test_get_img1()
    test_get_img2()
    test_remove_img1()
