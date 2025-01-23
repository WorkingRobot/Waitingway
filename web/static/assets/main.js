const main = document.querySelector('main');
const region_hyperlinks = document.querySelector('.region-hyperlinks');
const region_dropdown = document.querySelector('.region-dropdown');
const global_table = document.querySelector('#table-global');
const nav = document.querySelector('nav');
const theme_toggles = document.querySelectorAll('.theme-toggle');
const timer_container = document.querySelector('.timer-container');
const timer_group = timer_container.querySelector('.timer');
const timer_circle = timer_group.querySelector('circle');

function updateAnchorOffset(e) {
    document.documentElement.style.setProperty('--link-offset', `${nav.getBoundingClientRect().height}px`);
}

addEventListener("resize", updateAnchorOffset);
updateAnchorOffset();

function switchTheme(e) {
    toggleTheme();
    e.preventDefault();
}

for (let toggle of theme_toggles) {
    toggle.addEventListener('click', switchTheme);
}

function format_duration(duration) {
    if (duration == 0) {
        return 'Instant';
    }
    return humanizeDuration(duration, { largest: 2, maxDecimalPoints: 1 });
}

function format_relative_past(time) {
    let diff = Date.now() - Date.parse(time);
    if (diff < 1000) {
        return 'Just now';
    }
    return humanizeDuration(diff, { largest: 2, round: true }) + ' ago';
}

function format_future_duration(diff) {
    if (diff < 500) {
        return 'soon';
    }
    return 'in ' + humanizeDuration(diff, { largest: 2, round: true });
}

function update_global_row(row, text) {
    let data = row.querySelector('td');
    data.textContent = text;
}

function get_region_section_a(region_id) {
    let ret = region_hyperlinks.querySelector('#region-a-' + region_id);
    if (ret === null) {
        ret = create_hierarchy({
            "tag": "li",
            "id": "region-a-" + region_id,
            "class_name": "hide-mobile",
            "children": [
                {
                    "tag": "a",
                    "class_name": "secondary",
                    "attributes": { "href": "#" },
                    "content": "Region"
                }
            ]
        });
        region_hyperlinks.appendChild(ret);
    }
    return ret.querySelector('a');
}
function get_region_section_b(region_id) {
    let ret = region_dropdown.querySelector('#region-b-' + region_id);
    if (ret === null) {
        ret = create_hierarchy({
            "tag": "li",
            "id": "region-b-" + region_id,
            "children": [
                {
                    "tag": "a",
                    "attributes": { "href": "#" },
                    "content": "Region"
                }
            ]
        });
        ret.children[0].addEventListener('click', function (e) {
            region_dropdown.parentElement.removeAttribute('open');
        });
        region_dropdown.appendChild(ret);
    }
    return ret.querySelector('a');
}

function get_dc_section(datacenter_id) {
    let ret = main.querySelector('#dc-' + datacenter_id);
    if (ret === null) {
        ret = create_hierarchy({
            "tag": "section",
            "id": "dc-" + datacenter_id,
            "children": [
                {
                    "tag": "hgroup",
                    "children": [
                        {
                            "tag": "h2",
                            "class_name": "dc-name"
                        },
                        {
                            "tag": "h4",
                            "class_name": "region-name"
                        }
                    ]
                },
                {
                    "tag": "div",
                    "class_name": "worlds-container"
                }
            ]
        });
        main.appendChild(ret);
    }
    return ret;
}

function get_world_row(datacenter_id, world_id) {
    let dc_section = get_dc_section(datacenter_id);

    let ret = dc_section.querySelector('#world-' + world_id);
    if (ret === null) {
        ret = create_hierarchy({
            "tag": "div",
            "class_name": "shadow-container",
            "children": [
                {
                    "tag": "article",
                    "id": "world-" + world_id,
                    "children": [
                        {
                            "tag": "header",
                            "class_name": "world-header",
                            "children": [
                                {
                                    "tag": "h4",
                                    "class_name": "world-name"
                                },
                                {
                                    "tag": "h4",
                                    "class_name": "world-icons",
                                    "children": [
                                        {
                                            "tag": "span",
                                            "attributes": { "data-placement": "shifted" },
                                            "children": [
                                                {
                                                    "tag": "span",
                                                    "class_name": "status",
                                                }
                                            ]
                                        },
                                        {
                                            "tag": "span",
                                            "attributes": { "data-placement": "shifted-long" },
                                            "children": [
                                                {
                                                    "tag": "span",
                                                    "class_name": "status",
                                                }
                                            ]
                                        },
                                        {
                                            "tag": "span",
                                            "attributes": { "data-placement": "shifted-long" },
                                            "children": [
                                                {
                                                    "tag": "span",
                                                    "class_name": "status",
                                                }
                                            ]
                                        }
                                    ]
                                }
                            ]
                        },
                        {
                            "tag": "div",
                            "class_name": "world-body",
                            "attributes": { "data-placement": "bottom" },
                            "children": [
                                {
                                    "tag": "div",
                                    "children": [
                                        {
                                            "tag": "h6",
                                            "content": "Queue Time"
                                        },
                                        { "tag": "div", "class_name": "queue-time" }
                                    ]
                                },
                                {
                                    "tag": "div",
                                    "children": [
                                        {
                                            "tag": "h6",
                                            "content": "Queue Size"
                                        },
                                        { "tag": "div", "class_name": "queue-size" }
                                    ]
                                }
                            ]
                        }
                    ]
                }
            ]
        });
        dc_section.querySelector('.worlds-container').appendChild(ret);
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

function toggleModal(e) {
    e.preventDefault();
    const modal = document.getElementById(e.currentTarget.dataset.target);
    if (!modal) return;
    modal && (modal.open ? closeModal(modal) : openModal(modal));
};

const modalIsOpenClass = "modal-is-open";
const modalOpeningClass = "modal-is-opening";
const modalClosingClass = "modal-is-closing";
const scrollbarWidthCssVar = "--pico-scrollbar-width";
const modalAnimDur = 400;
const modalAnimDur2 = 400;
let visibleModal = null;
function openModal(modal) {
    const html = document.documentElement;
    const scrollbarWidth = window.innerWidth - document.documentElement.clientWidth;
    if (scrollbarWidth) {
        html.style.setProperty(scrollbarWidthCssVar, `${scrollbarWidth}px`);
    }
    html.classList.add(modalIsOpenClass);
    html.classList.add(modalOpeningClass);
    setTimeout(() => {
        visibleModal = modal;
    }, modalAnimDur);
    setTimeout(() => {
        html.classList.remove(modalOpeningClass);
    }, modalAnimDur2);
    modal.showModal();
};

function closeModal(modal) {
    visibleModal = null;
    const html = document.documentElement;
    html.classList.add(modalClosingClass);
    setTimeout(() => {
        html.classList.remove(modalClosingClass, modalIsOpenClass);
        html.style.removeProperty(scrollbarWidthCssVar);
        modal.close();
    }, modalAnimDur2);
};

document.addEventListener("click", (event) => {
    if (visibleModal === null) return;
    const modalContent = visibleModal.querySelector("article");
    const isClickInside = modalContent.contains(event.target);
    !isClickInside && closeModal(visibleModal);
});

document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && visibleModal) {
        closeModal(visibleModal);
    }
});

// Get scrollbar width
const getScrollbarWidth = () => {
    const scrollbarWidth = window.innerWidth - document.documentElement.clientWidth;
    return scrollbarWidth;
};

// Is scrollbar visible
const isScrollbarVisible = () => {
    return document.body.scrollHeight > screen.height;
};

const status_lookup = {
    1: ['Online', 'status-online'],
    2: ['Issues', 'status-issues']
    3: ['Offline', 'status-offline'],
};

const create_lookup = {
    true: ['Character Creation Available', 'status-create'],
    false: ['Character Creation Unavailable', 'status-congested']
};

const transfer_lookup = {
    false: ['DC Travel Allowed', 'status-transferrable'],
    true: ['DC Travel Prohibited', 'status-prohibited']
};

function update_world_data(data) {
    let row = get_world_row(data.datacenter_id, data.id);
    let status_list = row.querySelector(".world-icons").children;
    {
        let entry = status_lookup[data.world_status];
        status_list[0].setAttribute('data-tooltip', entry[0]);
        status_list[0].className = entry[1];
    }

    {
        let entry = create_lookup[data.world_character_creation_enabled];
        status_list[1].setAttribute('data-tooltip', entry[0]);
        status_list[1].className = entry[1];
    }

    {
        let entry = transfer_lookup[data.travel_prohibited];
        status_list[2].setAttribute('data-tooltip', entry[0]);
        status_list[2].className = entry[1];
    }

    row.querySelector('.world-name').textContent = data.name;
    row.querySelector('.queue-time').textContent = format_duration(data.queue_duration * 1000);
    row.querySelector('.queue-size').textContent = data.queue_size;
    row.querySelector('.world-body').setAttribute('data-tooltip', `Updated ${format_relative_past(data.queue_last_update)}`);
}

function update_dc_data(data, regions) {
    let dc_section = get_dc_section(data.id);
    dc_section.querySelector('.dc-name').textContent = data.name;
    dc_section.querySelector('.region-name').textContent = regions.find(region => region.id === data.region_id).name;
}

function update_region_data(data, datacenters) {
    for (let section of [get_region_section_a(data.id), get_region_section_b(data.id)]) {
        section.textContent = data.name;
        section.setAttribute('href', '#dc-' + datacenters.find(dc => dc.region_id === data.id).id);
    }
}

function update_global_data(data) {
    update_global_row(global_table.querySelector('#travel-time'), format_duration(data.average_travel_time * 1000));
}

function default_compare(a, b) {
    if (a > b) {
        return 1;
    }
    if (b > a) {
        return -1;
    }
    return 0;
}

function chain_compare(a, b) {
    if (a === 0) {
        return b;
    }
    return a;
}

function update_from_summary(summary) {
    update_global_data(summary);
    summary.regions.sort((a, b) => default_compare(a.id, b.id));
    summary.datacenters.sort((a, b) => chain_compare(default_compare(a.region_id, b.region_id), default_compare(a.id, b.id)));
    summary.worlds.sort((a, b) => default_compare(a.id, b.id));
    console.log(summary);

    for (let dc of summary.datacenters) {
        update_dc_data(dc, summary.regions);
    }
    for (let world of summary.worlds) {
        update_world_data(world);
    }
    for (let region of summary.regions) {
        update_region_data(region, summary.datacenters);
    }

    if (!scrolled_to_hash) {
        scrolled_to_hash = true;
        let hash = document.getElementById(window.location.hash.slice(1));
        if (hash !== null) {
            hash.scrollIntoView();
        }
    }
}

function update_from_url(url) {
    timer_group.classList.remove('timer-waiting');
    timer_group.classList.add('timer-reloading');

    let xhr = new XMLHttpRequest();
    xhr.open('GET', url, true);
    xhr.onload = function () {
        try {
            if (xhr.status === 200) {
                let data = JSON.parse(xhr.responseText);
                localStorage.setItem('summary', xhr.responseText);
                update_from_summary(data);
            }
        }
        finally {
            queue_url_update(url);
        }
    };
    xhr.onerror = function () {
        queue_url_update(url);
    };
    xhr.send();
}

function queue_url_update(url) {
    let now = Date.now();
    let timerId = setInterval(() => {
        // Don't update in the background
        if (document.visibilityState === 'hidden') {
            return;
        }
        let diff = Date.now() - now;
        if (diff >= 30000) {
            timer_circle.removeAttribute('stroke-dashoffset');
            timer_container.setAttribute('data-tooltip', `Reloading`);
            clearInterval(timerId);
            update_from_url(url);
        }
        else {
            let percent = diff / 30000;
            timer_circle.setAttribute('stroke-dashoffset', -9 * 2 * Math.PI * percent);
            timer_container.setAttribute('data-tooltip', `Reloading ${format_future_duration(30000 - diff)}`);
        }
    }, 100);
    timer_group.classList.remove('timer-reloading');
    timer_group.classList.add('timer-waiting');
}

let scrolled_to_hash = false;
try {
    let summary = localStorage.getItem('summary');
    if (summary !== null) {
        update_from_summary(JSON.parse(summary));
    }
}
catch (e) {
}
update_from_url('/api/v1/summary');