import requests

with open("./example.jpeg", "rb") as file:
    image = file.read()

r = requests.post(
    "http://127.0.0.1:8000/v1/user-profiles",
    params={"format": "jpeg"},
    headers={
        "content-length": str(len(image)),
        "content-type": "application/octet-stream"
    },
    data=image,
)

r.raise_for_status()
data = r.json()

print("My image id: {}", data['image_id'])
print("It took {}s to complete!", data['processing_time'])
print("And has a checksum of {}!", data['checksum'])
