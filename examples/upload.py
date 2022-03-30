import requests

with open("./example.jpeg", "rb") as file:
    image = file.read()

r = requests.post(
    "http://127.0.0.1:8000/v1/images/user-profiles",
    params={"format": "jpeg"},
    headers={
        "content-length": str(len(image)),
        "content-type": "application/octet-stream"
    },
    data=image,
)

r.raise_for_status()
data = r.json()

print(f"My image id: {data['image_id']}")
print(f"It took {data['processing_time']}s to complete!")
print(f"And has a checksum of {data['checksum']}!")
