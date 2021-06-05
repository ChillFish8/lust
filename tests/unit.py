import base64
import requests
import uuid

working_ids = {}


def get_base_data(file: str) -> str:
    with open(file, "rb") as file:
        data = file.read()
        print(f"original {len(data)}")
        return base64.standard_b64encode(data).decode("utf-8")


def test_png_upload1():
    global working_ids
    data = get_base_data("./samples/news.png")
    payload = {
        "format": "png",
        "data": data,
    }
    r = requests.post("http://127.0.0.1:7070/admin/create/image", json=payload)
    data = r.json()

    assert r.status_code == 200
    assert data['data']['category'] == "default"

    file_id = data['data']['file_id']
    working_ids['default'] = file_id
    print(file_id)


def test_get_img_default():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}")
    assert r.status_code == 200


def test_get_img_preset_webp():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}?format=webp")
    assert r.status_code == 200


def test_get_img_preset_png():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}?format=png")
    assert r.status_code == 200


def test_get_img_preset_jpeg():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}?format=jpeg")
    assert r.status_code == 200


def test_get_img_format_gif():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}?format=gif")
    assert r.status_code == 200


def test_get_img_preset_large():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}?preset=large")
    assert r.status_code == 200


def test_get_img_preset_medium():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}?preset=medium")
    assert r.status_code == 200


def test_get_img_preset_small():
    r = requests.get(f"http://127.0.0.1:7070/images/{working_ids['default']}?preset=small")
    assert r.status_code == 200


def test_get_nothing1():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}")
    assert r.status_code == 404


def test_get_nothing2():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}?format=png")
    assert r.status_code == 404


def test_get_nothing3():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}?format=jpeg")
    assert r.status_code == 404


def test_get_nothing4():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}?format=webp")
    assert r.status_code == 404


def test_get_nothing5():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}?format=gif")
    assert r.status_code == 404


def test_remove_img1():
    r = requests.delete(
        f"http://127.0.0.1:7070/admin/delete/image/44524a33-c505-476d-b23b-c42de1fd796a")
    print(r.content)
    assert r.status_code == 200


if __name__ == '__main__':
    test_png_upload1()
    test_get_img_default()
    test_get_nothing1()
    # test_remove_img1()
