import {ActionIcon, Group, Title} from "@mantine/core";
import {MdAdd, MdRefresh} from "react-icons/md";
import React from "react";
import {useRouter} from "next/router";

interface IParams {
    title: string;
    onAdd: (() => void) | null;
}

export default function AdminTopBar({title, onAdd}: IParams) {
    const router = useRouter()

    return <Group position={'apart'}>
        <Title order={2}>{title}</Title>

        <Group spacing={'lg'}>
            {onAdd &&
                <ActionIcon
                    variant={'filled'}
                    color={'green'}
                    radius={'xl'}
                    size={36}
                    onClick={onAdd}>
                    <MdAdd size={28}/>
                </ActionIcon>
            }

            <ActionIcon variant={'filled'} radius={'xl'} size={36} onClick={() => router.reload()}>
                <MdRefresh size={28}/>
            </ActionIcon>
        </Group>
    </Group>
}
