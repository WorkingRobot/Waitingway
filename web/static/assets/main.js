function format_duration(duration) {
    if (duration == 0) {
        return 'instant';
    }
    return humanizeDuration(duration);
}

function format_relative(time) {
    let diff = Date.now() - Date.parse(time);
    if (diff < 1000) {
        return 'just now';
    }
    return humanizeDuration(diff) + ' ago';
}

let main = document.querySelector('main');
let global_table = document.querySelector('#table-global');

function update_global_row(row, text) {
    let data = row.querySelector('td');
    data.textContent = text;
}

function get_dc_section(datacenter_id) {
    let ret = main.querySelector('section#dc-' + datacenter_id);
    if (ret === null) {
        ret = create_hierarchy({
            "tag": "section",
            "id": "dc-" + datacenter_id,
            "children": [
                {
                    "tag": "hgroup",
                    "children": [
                        {
                            "tag": "h3",
                            "class_name": "dc-name"
                        },
                        {
                            "tag": "h4",
                            "class_name": "region-name"
                        }
                    ]
                },
                {
                    "tag": "table",
                    "class_name": "iconified",
                    "children": [
                        {
                            "tag": "thead",
                            "children": [
                                {
                                    "tag": "tr",
                                    "children": [
                                        {
                                            "tag": "th"
                                        },
                                        {
                                            "tag": "th"
                                        },
                                        {
                                            "tag": "th"
                                        },
                                        {
                                            "tag": "th",
                                            "content": "Server"
                                        },
                                        {
                                            "tag": "th",
                                            "content": "Queue Size"
                                        },
                                        {
                                            "tag": "th",
                                            "content": "Queue Time"
                                        },
                                        {
                                            "tag": "th",
                                            "content": "Last Updated"
                                        }
                                    ]
                                }
                            ]
                        },
                        {
                            "tag": "tbody"
                        }
                    ]
                }
            ]
        });
        main.appendChild(ret);
    }
    return ret;
}

function get_world_row(datacenter_id, world_id) {
    let dc_section = get_dc_section(datacenter_id);

    let ret = dc_section.querySelector('tbody tr#world-' + world_id);
    if (ret === null) {
        ret = create_hierarchy({
            "tag": "tr",
            "id": "world-" + world_id,
            "children": [
                {
                    "tag": "td"
                },
                {
                    "tag": "td"
                },
                {
                    "tag": "td"
                },
                {
                    "tag": "th",
                    "attributes": {
                        "scope": "row"
                    }
                },
                {
                    "tag": "td"
                },
                {
                    "tag": "td"
                },
                {
                    "tag": "td"
                }
            ]
        });
        dc_section.querySelector('tbody').appendChild(ret);
    }
    return ret;
}

function create_hierarchy(data) {
    let tag = data.tag;
    let class_name = data.class_name;
    let id = data.id;
    let content = data.content;
    let attributes = data.attributes;
    let children = data.children;

    let ret = document.createElement(tag);
    if (class_name !== null && class_name !== undefined) {
        ret.classList.add(class_name);
    }
    if (id !== null && id !== undefined) {
        ret.id = id;
    }
    if (content !== null && content !== undefined) {
        ret.textContent = content;
    }
    if (attributes !== null && attributes !== undefined) {
        for (let key in attributes) {
            ret.setAttribute(key, attributes[key]);
        }
    }
    if (children !== null && children !== undefined) {
        for (let child of children) {
            ret.appendChild(create_hierarchy(child));
        }
    }
    return ret;
}

function update_world_data(data) {
    let row = get_world_row(data.datacenter_id, data.id);
    let data_list = row.children;

    {
        let status_class = '';
        switch (data.world_status) {
            case 1: status_class = 'status-online'; break;
            case 3: status_class = 'status-issues'; break;
            case 2: status_class = 'status-offline'; break;
        }
        data_list[0].className = status_class;
    }

    {
        let status_class = data.world_character_creation_enabled ? 'status-create' : 'status-congested';
        data_list[1].className = status_class;
    }

    {
        let status_class = data.travel_prohibited ? 'status-prohibited' : 'status-transferrable';
        data_list[2].className = status_class;
    }

    data_list[3].textContent = data.name;
    data_list[4].textContent = data.queue_size;
    data_list[5].textContent = format_duration(data.queue_duration * 1000);
    data_list[6].textContent = format_relative(data.queue_last_update);
}

function update_dc_data(data, regions) {
    let dc_section = get_dc_section(data.id);
    dc_section.querySelector('h3.dc-name').textContent = data.name;
    dc_section.querySelector('h4.region-name').textContent = regions.find(region => region.id === data.region_id).name;
}

function update_global_data(data) {
    update_global_row(global_table.querySelector('tr#travel-time'), data.average_travel_time);
}

function update_from_summary(summary) {
    update_global_data(summary);
    summary.datacenters.sort((a, b) => (a.region_id > b.region_id) ? 1 : ((b.id > a.id) ? -1 : 0));
    summary.worlds.sort((a, b) => (a.id > b.id) ? 1 : ((b.id > a.id) ? -1 : 0));

    for (let dc of summary.datacenters) {
        update_dc_data(dc, summary.regions);
    }
    for (let world of summary.worlds) {
        update_world_data(world);
    }
}

function update_from_url(url) {
    // XMLHttpRequest
    let xhr = new XMLHttpRequest();
    xhr.open('GET', url, true);
    xhr.onreadystatechange = function () {
        if (xhr.readyState === 4 && xhr.status === 200) {
            let data = JSON.parse(xhr.responseText);
            update_from_summary(data);
        }
    };
    xhr.send();
}
update_from_url('/api/v1/summary');