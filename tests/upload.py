import base64
import requests
import uuid


def get_base_data(file: str) -> str:
    with open(file, "rb") as file:
        return base64.urlsafe_b64encode(file.read()).decode("utf-8")


"""def test_png_upload1():
    data = get_base_data("./samples/news.png")
    payload = {
        ""
    }"""


def test_get_img1():
    r = requests.get(f"http://127.0.0.1:7070/images?file_id={uuid.uuid4()}")
    assert r.status_code == 404


def test_get_img2():
    r = requests.get(f"http://127.0.0.1:7070/images/{uuid.uuid4()}")
    assert r.status_code == 404