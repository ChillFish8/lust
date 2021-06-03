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
    r = requests.post("http://127.0.0.1:7070/admin/create/image", json=payload)
    print(r.content)
    assert r.status_code == 200


def test_get_img1():
    r = requests.get(f"http://127.0.0.1:7070/images/4463d548-9408-4762-a281-04c439db224c")
    assert r.status_code == 200


def test_get_img2():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}")
    assert r.status_code == 404


def test_remove_img1():
    r = requests.delete(f"http://127.0.0.1:7070/admin/delete/image/44524a33-c505-476d-b23b-c42de1fd796a")
    print(r.content)
    assert r.status_code == 200


if __name__ == '__main__':
    test_png_upload1()
    test_get_img1()
    test_get_img2()
    test_remove_img1()
